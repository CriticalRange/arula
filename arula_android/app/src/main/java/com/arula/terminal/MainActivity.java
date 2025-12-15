package com.arula.terminal;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.Editable;
import android.text.TextWatcher;
import android.util.Log;
import android.view.Menu;
import android.view.MenuItem;
import android.view.View;
import android.widget.EditText;
import android.widget.ImageButton;
import android.widget.LinearLayout;
import android.widget.TextView;

// Import custom UI components
import com.arula.terminal.ui.canvas.LivingBackground;
import com.arula.terminal.ui.canvas.LoadingSpinner;
import com.arula.terminal.ui.menu.SlidingMenuView;
import androidx.appcompat.app.AppCompatActivity;
import androidx.appcompat.widget.Toolbar;
import androidx.lifecycle.ViewModelProvider;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import com.arula.terminal.databinding.ActivityMainBinding;
import com.google.android.material.snackbar.Snackbar;
import org.json.JSONException;
import org.json.JSONObject;
import java.util.List;

/**
 * Main activity for Arula Terminal
 */
public class MainActivity extends AppCompatActivity implements ArulaNative.ArulaCallback {
    private static final String TAG = "MainActivity";
    private static final int REQUEST_SETTINGS = 1001;

    private ActivityMainBinding binding;
    private MessageAdapter messageAdapter;
    private MainViewModel viewModel;
    private Handler mainHandler;

    // Advanced UI components
    private LivingBackground livingBackground;
    private LoadingSpinner typingSpinner;
    private SlidingMenuView slidingMenu;
    private ImageButton menuButton;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        binding = ActivityMainBinding.inflate(getLayoutInflater());
        setContentView(binding.getRoot());

        // Initialize main handler for UI updates
        mainHandler = new Handler(Looper.getMainLooper());

        // Setup toolbar
        Toolbar toolbar = binding.toolbar;
        setSupportActionBar(toolbar);

        // Initialize ViewModel
        viewModel = new ViewModelProvider(this).get(MainViewModel.class);

        // Setup RecyclerView for messages
        setupMessageList();

        // Setup input field
        setupInputField();

        // Setup menu button
        menuButton = binding.getRoot().findViewById(R.id.menuButton);
        menuButton.setOnClickListener(v -> toggleMenu());

        // Setup send button
        binding.sendButton.setOnClickListener(v -> sendMessage());

        // Initialize advanced UI components
        initializeAdvancedUI();

        // Initialize Arula core
        initializeArula();
    }

    private void setupMessageList() {
        messageAdapter = new MessageAdapter();
        binding.messageList.setLayoutManager(new LinearLayoutManager(this));
        binding.messageList.setAdapter(messageAdapter);

        // Scroll to bottom when new messages are added
        messageAdapter.registerAdapterDataObserver(new RecyclerView.AdapterDataObserver() {
            @Override
            public void onItemRangeInserted(int positionStart, int itemCount) {
                binding.messageList.smoothScrollToPosition(messageAdapter.getItemCount() - 1);
            }
        });
    }

    private void setupInputField() {
        EditText inputField = binding.messageInput;

        // Enable send button when there's text
        inputField.addTextChangedListener(new TextWatcher() {
            @Override
            public void beforeTextChanged(CharSequence s, int start, int count, int after) {}

            @Override
            public void onTextChanged(CharSequence s, int start, int before, int count) {
                binding.sendButton.setEnabled(s.toString().trim().length() > 0);
            }

            @Override
            public void afterTextChanged(Editable s) {}
        });

        // Send on Ctrl+Enter or Enter if not multiline
        inputField.setOnEditorActionListener((v, actionId, event) -> {
            if (event != null && event.getAction() == android.view.KeyEvent.ACTION_DOWN) {
                if (event.getKeyCode() == android.view.KeyEvent.KEYCODE_ENTER &&
                    (event.isShiftPressed() || event.isCtrlPressed())) {
                    sendMessage();
                    return true;
                }
            }
            return false;
        });
    }

    private void initializeArula() {
        // Load configuration
        JSONObject config = viewModel.getConfig();

        // Initialize native library
        ArulaNative.setCallback(this);
        boolean initialized = ArulaNative.initializeWithContext(this, config.toString());

        if (!initialized) {
            showError("Failed to initialize Arula core");
        } else {
            // Load conversation history
            List<Message> history = viewModel.getMessages().getValue();
            if (history != null) {
                messageAdapter.setMessages(history);
            }
        }
    }

    private void sendMessage() {
        EditText inputField = binding.messageInput;
        String message = inputField.getText().toString().trim();

        if (message.isEmpty()) return;

        // Clear input
        inputField.setText("");

        // Add user message to UI
        Message userMessage = new Message(message, Message.Type.USER);
        messageAdapter.addMessage(userMessage);
        viewModel.addMessage(userMessage);

        // Show typing indicator
        showTypingIndicator(true);

        // Send to AI
        ArulaNative.sendMessage(message);
    }

    @Override
    public void onMessage(String message) {
        mainHandler.post(() -> {
            showTypingIndicator(false);
            Message aiMessage = new Message(message, Message.Type.ASSISTANT);
            messageAdapter.addMessage(aiMessage);
            viewModel.addMessage(aiMessage);
        });
    }

    @Override
    public void onStreamChunk(String chunk) {
        mainHandler.post(() -> {
            // Update last message with streaming chunk
            messageAdapter.appendToLastMessage(chunk);
        });
    }

    @Override
    public void onToolStart(String toolName, String toolId) {
        mainHandler.post(() -> {
            // Show tool execution indicator
            Message toolMessage = new Message("🔧 " + toolName + "...", Message.Type.TOOL);
            toolMessage.setToolId(toolId);
            messageAdapter.addMessage(toolMessage);
        });
    }

    @Override
    public void onToolComplete(String toolId, String result) {
        mainHandler.post(() -> {
            // Update tool message with result
            messageAdapter.updateToolMessage(toolId, result);
        });
    }

    @Override
    public void onError(String error) {
        mainHandler.post(() -> {
            showTypingIndicator(false);
            showError(error);
            Message errorMessage = new Message("Error: " + error, Message.Type.ERROR);
            messageAdapter.addMessage(errorMessage);
        });
    }

    private void initializeAdvancedUI() {
        // Initialize living background
        livingBackground = binding.getRoot().findViewById(R.id.livingBackground);
        livingBackground.setEnabled(true);
        livingBackground.setOpacity(0.5f); // Semi-transparent by default

        // Initialize typing indicator spinner
        LinearLayout typingIndicator = binding.getRoot().findViewById(R.id.typingIndicator);
        typingSpinner = typingIndicator.findViewById(R.id.typingSpinner);
        typingSpinner.setAnimationSpeed(1.5f);

        // Initialize sliding menu
        slidingMenu = binding.getRoot().findViewById(R.id.slidingMenu);
        slidingMenu.setListener(new SlidingMenuView.MenuListener() {
            @Override
            public void onMenuOpened() {
                // Dim living background when menu is open
                livingBackground.setOpacity(0.2f);
            }

            @Override
            public void onMenuClosed() {
                // Restore living background opacity
                livingBackground.setOpacity(0.5f);
            }

            @Override
            public void onPageChanged(SlidingMenuView.MenuPage page) {
                Log.d(TAG, "Menu page changed to: " + page);
            }
        });
    }

    private void toggleMenu() {
        if (slidingMenu.isOpen()) {
            slidingMenu.closeMenu();
        } else {
            slidingMenu.openMenu();
        }
    }

    private void showTypingIndicator(boolean show) {
        LinearLayout typingIndicator = binding.getRoot().findViewById(R.id.typingIndicator);
        if (show) {
            typingIndicator.setVisibility(View.VISIBLE);
            typingSpinner.show();
            // Apply pulse animation
            typingIndicator.startAnimation(android.view.animation.AnimationUtils.loadAnimation(
                this, R.anim.typing_indicator_pulse));
        } else {
            typingSpinner.hide();
            typingIndicator.setVisibility(View.GONE);
            typingIndicator.clearAnimation();
        }
    }

    private void showError(String error) {
        Snackbar.make(binding.coordinator, error, Snackbar.LENGTH_LONG)
            .setAction("Dismiss", v -> {})
            .show();
    }

    @Override
    public boolean onCreateOptionsMenu(Menu menu) {
        getMenuInflater().inflate(R.menu.menu_main, menu);
        return true;
    }

    @Override
    public boolean onOptionsItemSelected(MenuItem item) {
        int id = item.getItemId();

        if (id == R.id.action_settings) {
            openSettings();
            return true;
        } else if (id == R.id.action_clear) {
            clearConversation();
            return true;
        } else if (id == R.id.action_export) {
            exportConversation();
            return true;
        }

        return super.onOptionsItemSelected(item);
    }

    private void openSettings() {
        Intent intent = new Intent(this, SettingsActivity.class);
        startActivityForResult(intent, REQUEST_SETTINGS);
    }

    private void clearConversation() {
        viewModel.clearMessages();
        messageAdapter.clearMessages();
    }

    private void exportConversation() {
        try {
            String exported = viewModel.exportConversation();
            // TODO: Implement share intent
            showError("Export feature coming soon");
        } catch (Exception e) {
            showError("Failed to export: " + e.getMessage());
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);

        if (requestCode == REQUEST_SETTINGS && resultCode == RESULT_OK) {
            // Configuration changed, reinitialize
            initializeArula();
        }
    }

    // Menu click handlers
    public void onConversationsClick(View v) {
        slidingMenu.navigateToPage(SlidingMenuView.MenuPage.CONVERSATIONS);
    }

    public void onSettingsClick(View v) {
        slidingMenu.navigateToPage(SlidingMenuView.MenuPage.SETTINGS);
    }

    public void onAboutClick(View v) {
        slidingMenu.navigateToPage(SlidingMenuView.MenuPage.ABOUT);
    }

    @Override
    public void onBackPressed() {
        if (slidingMenu.isOpen()) {
            if (slidingMenu.getCurrentPage() != SlidingMenuView.MenuPage.MAIN) {
                slidingMenu.navigateToMain();
            } else {
                slidingMenu.closeMenu();
            }
        } else {
            super.onBackPressed();
        }
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        ArulaNative.cleanup();
        binding = null;
    }
}
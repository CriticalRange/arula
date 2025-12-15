package com.arula.terminal;

import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;
import androidx.annotation.NonNull;
import androidx.recyclerview.widget.DiffUtil;
import androidx.recyclerview.widget.ListAdapter;
import androidx.recyclerview.widget.RecyclerView;
import com.arula.terminal.databinding.ItemMessageNeonBinding;
import java.text.SimpleDateFormat;
import java.util.*;

/**
 * Adapter for displaying chat messages
 */
public class MessageAdapter extends ListAdapter<Message, MessageAdapter.MessageViewHolder> {
    private static final DiffUtil.ItemCallback<Message> DIFF_CALLBACK = new DiffUtil.ItemCallback<Message>() {
        @Override
        public boolean areItemsTheSame(@NonNull Message oldItem, @NonNull Message newItem) {
            return oldItem.getId() == newItem.getId();
        }

        @Override
        public boolean areContentsTheSame(@NonNull Message oldItem, @NonNull Message newItem) {
            return oldItem.equals(newItem);
        }
    };

    private final Map<String, Integer> toolMessagePositions = new HashMap<>();
    private final SimpleDateFormat timeFormat = new SimpleDateFormat("HH:mm", Locale.getDefault());
    private final List<Message> messageList = new ArrayList<>();

    public MessageAdapter() {
        super(DIFF_CALLBACK);
    }

    @NonNull
    @Override
    public MessageViewHolder onCreateViewHolder(@NonNull ViewGroup parent, int viewType) {
        LayoutInflater inflater = LayoutInflater.from(parent.getContext());
        // Use the new neon layout
        ItemMessageNeonBinding binding = ItemMessageNeonBinding.inflate(inflater, parent, false);
        return new MessageViewHolder(binding);
    }

    @Override
    public void onBindViewHolder(@NonNull MessageViewHolder holder, int position) {
        Message message = getItem(position);
        holder.bind(message);
    }

    public void setMessages(List<Message> messages) {
        messageList.clear();
        messageList.addAll(messages);
        submitList(new ArrayList<>(messageList));
    }

    public void addMessage(Message message) {
        messageList.add(message);
        submitList(new ArrayList<>(messageList));
    }

    public void clearMessages() {
        messageList.clear();
        submitList(new ArrayList<>(messageList));
    }

    public void appendToLastMessage(String chunk) {
        int lastPosition = getItemCount() - 1;
        if (lastPosition >= 0) {
            Message lastMessage = getItem(lastPosition);
            if (lastMessage.getType() == Message.Type.ASSISTANT) {
                lastMessage.appendText(chunk);
                notifyItemChanged(lastPosition);
            }
        }
    }

    public void updateToolMessage(String toolId, String result) {
        Integer position = toolMessagePositions.get(toolId);
        if (position != null && position < getItemCount()) {
            Message message = getItem(position);
            message.appendText("\n" + result);
            notifyItemChanged(position);
        }
    }

    static class MessageViewHolder extends RecyclerView.ViewHolder {
        private final ItemMessageNeonBinding binding;

        public MessageViewHolder(ItemMessageNeonBinding binding) {
            super(binding.getRoot());
            this.binding = binding;
        }

        public void bind(Message message) {
            // Set message content
            binding.messageText.setText(message.getText());

            android.content.Context ctx = itemView.getContext();

            // Set message appearance based on type
            switch (message.getType()) {
                case USER:
                    binding.messageCard.setCardBackgroundColor(
                        ctx.getResources().getColor(R.color.neon_accent, null));
                    binding.senderText.setText("You");
                    binding.senderText.setTextColor(
                        ctx.getResources().getColor(R.color.neon_text, null));
                    binding.senderIndicator.setBackgroundColor(
                        ctx.getResources().getColor(R.color.neon_success, null));
                    // Apply gradient background
                    binding.messageCard.setBackgroundResource(R.drawable.bg_user_message);
                    break;

                case ASSISTANT:
                    binding.messageCard.setCardBackgroundColor(
                        ctx.getResources().getColor(R.color.neon_surface_raised, null));
                    binding.senderText.setText("Arula");
                    binding.senderText.setTextColor(
                        ctx.getResources().getColor(R.color.neon_text, null));
                    binding.senderIndicator.setBackgroundColor(
                        ctx.getResources().getColor(R.color.neon_accent, null));
                    // Apply surface background
                    binding.messageCard.setBackgroundResource(R.drawable.bg_assistant_message);
                    break;

                case TOOL:
                    binding.messageCard.setCardBackgroundColor(
                        ctx.getResources().getColor(R.color.neon_tool_bubble, null));
                    binding.senderText.setText("Tool");
                    binding.senderText.setTextColor(
                        ctx.getResources().getColor(R.color.neon_text, null));
                    binding.senderIndicator.setBackgroundColor(
                        ctx.getResources().getColor(R.color.neon_success, null));
                    // Apply tool background
                    binding.messageCard.setBackgroundResource(R.drawable.bg_tool_message);
                    break;

                case ERROR:
                    binding.messageCard.setCardBackgroundColor(
                        ctx.getResources().getColor(R.color.neon_danger, null));
                    binding.senderText.setText("Error");
                    binding.senderText.setTextColor(
                        ctx.getResources().getColor(R.color.neon_text, null));
                    binding.senderIndicator.setBackgroundColor(
                        ctx.getResources().getColor(R.color.neon_danger, null));
                    break;
            }

            // Set neon styling for text
            binding.messageText.setTextColor(
                ctx.getResources().getColor(R.color.neon_text, null));
            binding.timestampText.setTextColor(
                ctx.getResources().getColor(R.color.neon_muted, null));

            // Set timestamp
            if (message.getTimestamp() > 0) {
                String time = new SimpleDateFormat("HH:mm", Locale.getDefault())
                    .format(new Date(message.getTimestamp()));
                binding.timestampText.setText(time);
            } else {
                binding.timestampText.setText("");
            }

            // Add neon glow effect for messages
            if (message.getType() == Message.Type.USER || message.getType() == Message.Type.ASSISTANT) {
                binding.messageCard.setElevation(4f);
            } else {
                binding.messageCard.setElevation(0f);
            }
        }
    }
}
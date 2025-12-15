package com.arula.terminal.ui.menu;

import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.RectF;
import android.util.AttributeSet;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import androidx.annotation.Nullable;

import com.arula.terminal.R;
import com.arula.terminal.ui.animation.SpringAnimation;
import com.arula.terminal.ui.canvas.LiquidMenuBackground;

/**
 * Sliding menu with liquid background and spring animations
 * Replicates the desktop version's menu system
 */
public class SlidingMenuView extends FrameLayout {
    public enum MenuState {
        CLOSED,
        OPENING,
        OPEN,
        CLOSING
    }

    public enum MenuPage {
        MAIN,
        CONVERSATIONS,
        SETTINGS,
        ABOUT
    }

    private MenuState currentState = MenuState.CLOSED;
    private MenuPage currentPage = MenuPage.MAIN;

    // Animation components
    private SpringAnimation menuSpring;
    private SpringAnimation pageSpring;
    private ValueAnimator menuAnimator;
    private ValueAnimator pageAnimator;

    // Visual components
    private LiquidMenuBackground liquidBackground;
    private View menuContent;
    private ViewGroup mainMenuContainer;
    private ViewGroup conversationsContainer;
    private ViewGroup settingsContainer;
    private ViewGroup aboutContainer;

    // Animation parameters
    private float menuPosition = 0f; // 0 = closed, 1 = open
    private float pagePosition = 0f; // For page transitions
    private float menuWidth = 0f;
    private float pageSlideDistance = 0f;

    // Visual settings
    private int backgroundColor;
    private int accentColor;
    private Paint overlayPaint;
    private Path clipPath;
    private RectF clipBounds;

    public interface MenuListener {
        void onMenuOpened();
        void onMenuClosed();
        void onPageChanged(MenuPage page);
    }

    private MenuListener listener;

    public SlidingMenuView(Context context) {
        super(context);
        init();
    }

    public SlidingMenuView(Context context, @Nullable AttributeSet attrs) {
        super(context, attrs);
        init();
    }

    public SlidingMenuView(Context context, @Nullable AttributeSet attrs, int defStyleAttr) {
        super(context, attrs, defStyleAttr);
        init();
    }

    private void init() {
        backgroundColor = getContext().getColor(R.color.neon_background);
        accentColor = getContext().getColor(R.color.neon_accent);

        // Initialize animations
        menuSpring = new SpringAnimation(250f, 0.85f);
        pageSpring = new SpringAnimation(200f, 0.9f);

        // Initialize visual components
        overlayPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
        overlayPaint.setColor(backgroundColor);
        clipPath = new Path();
        clipBounds = new RectF();

        // Create liquid background
        liquidBackground = new LiquidMenuBackground(getContext());
        addView(liquidBackground, new LayoutParams(
            LayoutParams.MATCH_PARENT,
            LayoutParams.MATCH_PARENT
        ));

        // Inflate menu content
        LayoutInflater.from(getContext()).inflate(R.layout.sliding_menu_content, this, true);
        menuContent = findViewById(R.id.menuContent);
        mainMenuContainer = findViewById(R.id.mainMenuContainer);
        conversationsContainer = findViewById(R.id.conversationsContainer);
        settingsContainer = findViewById(R.id.settingsContainer);
        aboutContainer = findViewById(R.id.aboutContainer);

        // Initially hide
        setVisibility(View.GONE);
        setAlpha(0f);
    }

    @Override
    protected void onSizeChanged(int w, int h, int oldw, int oldh) {
        super.onSizeChanged(w, h, oldw, oldh);
        menuWidth = w * 0.8f; // Menu takes 80% of screen width
        pageSlideDistance = w;

        // Position menu content off-screen initially
        if (menuContent != null) {
            menuContent.setTranslationX(-menuWidth);
        }

        // Update clip bounds
        clipBounds.set(0, 0, menuWidth, h);
    }

    /**
     * Opens the sliding menu
     */
    public void openMenu() {
        if (currentState == MenuState.OPEN || currentState == MenuState.OPENING) return;

        currentState = MenuState.OPENING;
        setVisibility(View.VISIBLE);
        animateAlpha(1f);

        // Start liquid background animation
        liquidBackground.openMenu();

        // Start menu slide animation
        menuSpring.setTarget(1f);
        startMenuAnimation();

        // Initialize containers
        setupContainers();
    }

    /**
     * Closes the sliding menu
     */
    public void closeMenu() {
        if (currentState == MenuState.CLOSED || currentState == MenuState.CLOSING) return;

        currentState = MenuState.CLOSING;

        // Start liquid background animation
        liquidBackground.closeMenu();

        // Start menu slide animation
        menuSpring.setTarget(0f);
        startMenuAnimation();
    }

    /**
     * Navigates to a specific menu page
     */
    public void navigateToPage(MenuPage page) {
        if (page == currentPage) return;

        currentPage = page;
        pageSpring.setPosition(0f);
        pageSpring.setTarget(1f);
        startPageAnimation();

        if (listener != null) {
            listener.onPageChanged(page);
        }
    }

    /**
     * Goes back to the main page
     */
    public void navigateToMain() {
        navigateToPage(MenuPage.MAIN);
    }

    private void setupContainers() {
        // Hide all containers initially
        mainMenuContainer.setVisibility(currentPage == MenuPage.MAIN ? View.VISIBLE : View.INVISIBLE);
        conversationsContainer.setVisibility(currentPage == MenuPage.CONVERSATIONS ? View.VISIBLE : View.INVISIBLE);
        settingsContainer.setVisibility(currentPage == MenuPage.SETTINGS ? View.VISIBLE : View.INVISIBLE);
        aboutContainer.setVisibility(currentPage == MenuPage.ABOUT ? View.VISIBLE : View.INVISIBLE);

        // Position containers for page transitions
        for (int i = 0; i < getChildCount(); i++) {
            View child = getChildAt(i);
            if (child instanceof ViewGroup && child != menuContent && child != liquidBackground) {
                child.setTranslationX(i == getPageIndex(currentPage) ? 0f : pageSlideDistance);
            }
        }
    }

    private int getPageIndex(MenuPage page) {
        switch (page) {
            case MAIN: return 0;
            case CONVERSATIONS: return 1;
            case SETTINGS: return 2;
            case ABOUT: return 3;
            default: return 0;
        }
    }

    private void startMenuAnimation() {
        if (menuAnimator != null) {
            menuAnimator.cancel();
        }

        menuAnimator = ValueAnimator.ofFloat(0f, 1f);
        menuAnimator.setDuration(16); // ~60fps
        menuAnimator.setRepeatCount(ValueAnimator.INFINITE);

        menuAnimator.addUpdateListener(animation -> {
            boolean stillAnimating = menuSpring.update();
            menuPosition = menuSpring.getPosition();

            // Update menu content position
            if (menuContent != null) {
                float targetX = -menuWidth + (menuWidth * menuPosition);
                menuContent.setTranslationX(targetX);
            }

            // Update overlay alpha
            if (overlayPaint != null) {
                int alpha = (int) (100 * (1f - menuPosition));
                overlayPaint.setAlpha(alpha);
            }

            if (!stillAnimating) {
                animation.cancel();
                if (menuPosition > 0.5f) {
                    currentState = MenuState.OPEN;
                    if (listener != null) {
                        listener.onMenuOpened();
                    }
                } else {
                    currentState = MenuState.CLOSED;
                    animateAlpha(0f);
                    setVisibility(View.GONE);
                    if (listener != null) {
                        listener.onMenuClosed();
                    }
                }
            }

            invalidate();
        });

        menuAnimator.start();
    }

    private void startPageAnimation() {
        if (pageAnimator != null) {
            pageAnimator.cancel();
        }

        pageAnimator = ValueAnimator.ofFloat(0f, 1f);
        pageAnimator.setDuration(16); // ~60fps
        pageAnimator.setRepeatCount(ValueAnimator.INFINITE);

        pageAnimator.addUpdateListener(animation -> {
            boolean stillAnimating = pageSpring.update();
            pagePosition = pageSpring.getPosition();

            updatePagePositions();

            if (!stillAnimating) {
                animation.cancel();
                setupContainers();
            }

            invalidate();
        });

        pageAnimator.start();
    }

    private void updatePagePositions() {
        int currentIndex = getPageIndex(currentPage);

        for (int i = 0; i < getChildCount(); i++) {
            View child = getChildAt(i);
            if (child instanceof ViewGroup && child != menuContent && child != liquidBackground) {
                float offset = (i - currentIndex) * pageSlideDistance;
                float targetX = offset * (1f - pagePosition);
                child.setTranslationX(targetX);
                child.setAlpha(i == currentIndex ? 1f : 0.3f * (1f - pagePosition));
            }
        }
    }

    private void animateAlpha(float targetAlpha) {
        animate()
            .alpha(targetAlpha)
            .setDuration(200)
            .start();
    }

    @Override
    protected void dispatchDraw(Canvas canvas) {
        // Draw overlay
        if (menuPosition > 0f && menuPosition < 1f) {
            overlayPaint.setAlpha((int) (100 * (1f - menuPosition)));
            canvas.drawRect(0, 0, getWidth(), getHeight(), overlayPaint);
        }

        // Apply clip for menu content
        clipPath.reset();
        clipPath.addRect(clipBounds, Path.Direction.CW);
        canvas.clipPath(clipPath);

        super.dispatchDraw(canvas);
    }

    // Getters and setters
    public MenuState getCurrentState() { return currentState; }
    public MenuPage getCurrentPage() { return currentPage; }
    public boolean isOpen() { return currentState == MenuState.OPEN; }
    public void setListener(MenuListener listener) { this.listener = listener; }

    @Override
    protected void onDetachedFromWindow() {
        super.onDetachedFromWindow();
        if (menuAnimator != null) {
            menuAnimator.cancel();
        }
        if (pageAnimator != null) {
            pageAnimator.cancel();
        }
    }
}
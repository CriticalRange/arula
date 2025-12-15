package com.arula.terminal.ui.canvas;

import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.RadialGradient;
import android.graphics.Shader;
import android.util.AttributeSet;
import android.view.View;
import androidx.annotation.Nullable;

import com.arula.terminal.R;
import com.arula.terminal.ui.animation.SpringAnimation;

/**
 * Liquid expanding menu background animation
 * Replicates the desktop version's liquid menu effect
 */
public class LiquidMenuBackground extends View {
    private SpringAnimation springAnimation;
    private Paint liquidPaint;
    private Paint backgroundPaint;
    private Path liquidPath;

    private int accentColor;
    private int backgroundColor;
    private float centerX = 40f;
    private float centerY;
    private float maxRadius;
    private float currentRadius = 0f;

    public LiquidMenuBackground(Context context) {
        super(context);
        init();
    }

    public LiquidMenuBackground(Context context, @Nullable AttributeSet attrs) {
        super(context, attrs);
        init();
    }

    public LiquidMenuBackground(Context context, @Nullable AttributeSet attrs, int defStyleAttr) {
        super(context, attrs, defStyleAttr);
        init();
    }

    private void init() {
        // Initialize spring animation
        springAnimation = new SpringAnimation(200f, 0.85f);
        springAnimation.setListener(new SpringAnimation.SpringListener() {
            @Override
            public void onAnimationUpdate(float position, float velocity) {
                updateLiquidPosition(position);
            }

            @Override
            public void onAnimationComplete() {
                invalidate();
            }
        });

        // Initialize colors
        accentColor = getContext().getColor(R.color.neon_accent);
        backgroundColor = getContext().getColor(R.color.neon_background);

        // Initialize paints
        liquidPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
        liquidPaint.setStyle(Paint.Style.FILL);

        backgroundPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
        backgroundPaint.setColor(backgroundColor);

        liquidPath = new Path();

        // Start with menu closed
        setVisibility(View.INVISIBLE);
    }

    @Override
    protected void onSizeChanged(int w, int h, int oldw, int oldh) {
        super.onSizeChanged(w, h, oldw, oldh);
        centerY = h - 40f;
        maxRadius = Math.max(w, h) * 1.8f;
        updateLiquidPosition(springAnimation.getPosition());
    }

    /**
     * Opens the liquid menu with animation
     */
    public void openMenu() {
        setVisibility(View.VISIBLE);
        springAnimation.setTarget(1.0f);
        animateSpring();
    }

    /**
     * Closes the liquid menu with animation
     */
    public void closeMenu() {
        springAnimation.setTarget(0.0f);
        animateSpring();
    }

    /**
     * Instantly sets menu state without animation
     */
    public void setMenuOpen(boolean open, boolean animate) {
        if (open && getVisibility() != View.VISIBLE) {
            setVisibility(View.VISIBLE);
        }

        if (animate) {
            springAnimation.setTarget(open ? 1.0f : 0.0f);
            animateSpring();
        } else {
            springAnimation.setPosition(open ? 1.0f : 0.0f);
            updateLiquidPosition(open ? 1.0f : 0.0f);
            if (!open) {
                setVisibility(View.INVISIBLE);
            }
        }
    }

    private void animateSpring() {
        ValueAnimator animator = ValueAnimator.ofFloat(0f, 1f);
        animator.setDuration(16); // ~60fps updates
        animator.setRepeatCount(ValueAnimator.INFINITE);

        animator.addUpdateListener(animation -> {
            boolean stillAnimating = springAnimation.update();
            if (!stillAnimating) {
                animation.cancel();
                if (springAnimation.getTarget() < 0.01f) {
                    setVisibility(View.INVISIBLE);
                }
            }
        });

        animator.start();
    }

    private void updateLiquidPosition(float position) {
        if (position < 0.01f) {
            currentRadius = 0f;
        } else {
            currentRadius = maxRadius * position;

            // Create gradient with opacity based on position
            int alpha = (int) (0.98f * 255 * Math.min(position, 1.0f));
            int colorWithAlpha = Color.argb(alpha,
                Color.red(accentColor),
                Color.green(accentColor),
                Color.blue(accentColor));

            // Apply radial gradient for glow effect
            RadialGradient gradient = new RadialGradient(
                centerX, centerY, currentRadius,
                new int[]{colorWithAlpha, Color.TRANSPARENT},
                new float[]{0.7f, 1.0f},
                Shader.TileMode.CLAMP
            );
            liquidPaint.setShader(gradient);
        }
        invalidate();
    }

    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);

        // Draw background
        canvas.drawRect(0, 0, getWidth(), getHeight(), backgroundPaint);

        // Draw liquid circle if visible
        if (currentRadius > 0) {
            liquidPath.reset();
            liquidPath.addCircle(centerX, centerY, currentRadius, Path.Direction.CW);
            canvas.drawPath(liquidPath, liquidPaint);
        }
    }
}
package com.arula.terminal.ui.canvas;

import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.DashPathEffect;
import android.graphics.Paint;
import android.graphics.PathEffect;
import android.graphics.RectF;
import android.util.AttributeSet;
import android.view.View;
import androidx.annotation.Nullable;

import com.arula.terminal.R;

/**
 * Advanced loading spinner with orbital animation
 * Replicates the desktop version's loading spinner
 */
public class LoadingSpinner extends View {
    private enum SpinnerType {
        ORBITAL,    // Rotating dots
        PULSE,      // Pulsing circle
        WAVE        // Wave effect
    }

    private SpinnerType spinnerType = SpinnerType.ORBITAL;
    private Paint circlePaint;
    private Paint dotPaint;
    private Paint glowPaint;
    private RectF circleBounds;

    private int accentColor;
    private int glowColor;
    private float rotation = 0f;
    private float pulseScale = 0f;
    private float waveOffset = 0f;
    private ValueAnimator animator;

    // Orbital spinner properties
    private int dotCount = 8;
    private float dotRadius = 6f;
    private float orbitRadius = 30f;

    // Animation speed
    private float animationSpeed = 1f;
    private boolean isAnimating = false;

    public LoadingSpinner(Context context) {
        super(context);
        init();
    }

    public LoadingSpinner(Context context, @Nullable AttributeSet attrs) {
        super(context, attrs);
        init();
    }

    public LoadingSpinner(Context context, @Nullable AttributeSet attrs, int defStyleAttr) {
        super(context, attrs, defStyleAttr);
        init();
    }

    private void init() {
        accentColor = getContext().getColor(R.color.neon_accent);
        glowColor = getContext().getColor(R.color.neon_glow);

        // Initialize paints
        circlePaint = new Paint(Paint.ANTI_ALIAS_FLAG);
        circlePaint.setStyle(Paint.Style.STROKE);
        circlePaint.setStrokeWidth(2f);
        circlePaint.setColor(accentColor);
        circlePaint.setAlpha(100);

        dotPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
        dotPaint.setStyle(Paint.Style.FILL);
        dotPaint.setColor(accentColor);

        glowPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
        glowPaint.setStyle(Paint.Style.FILL);
        glowPaint.setColor(glowColor);
        glowPaint.setAlpha(50);

        circleBounds = new RectF();

        // Hide by default
        setVisibility(View.GONE);
    }

    @Override
    protected void onSizeChanged(int w, int h, int oldw, int oldh) {
        super.onSizeChanged(w, h, oldw, oldh);
        float centerX = w / 2f;
        float centerY = h / 2f;
        float radius = Math.min(w, h) / 2f - 20f;
        circleBounds.set(centerX - radius, centerY - radius, centerX + radius, centerY + radius);
        orbitRadius = radius * 0.8f;
    }

    /**
     * Sets the spinner type
     */
    public void setSpinnerType(SpinnerType type) {
        this.spinnerType = type;
        if (isAnimating) {
            stopAnimation();
            startAnimation();
        }
    }

    /**
     * Sets the animation speed multiplier
     */
    public void setAnimationSpeed(float speed) {
        this.animationSpeed = speed;
    }

    /**
     * Shows and starts the spinner animation
     */
    public void show() {
        if (getVisibility() != View.VISIBLE) {
            setVisibility(View.VISIBLE);
            startAnimation();
        }
    }

    /**
     * Hides and stops the spinner animation
     */
    public void hide() {
        stopAnimation();
        setVisibility(View.GONE);
    }

    private void startAnimation() {
        if (animator != null) return;

        isAnimating = true;
        animator = ValueAnimator.ofFloat(0f, 1f);
        animator.setDuration((long) (16 / animationSpeed)); // ~60fps adjusted by speed
        animator.setRepeatCount(ValueAnimator.INFINITE);

        animator.addUpdateListener(animation -> {
            switch (spinnerType) {
                case ORBITAL:
                    rotation += 0.05f * animationSpeed;
                    break;
                case PULSE:
                    pulseScale = (float) Math.sin(animation.getAnimatedFraction() * Math.PI * 2) * 0.3f + 0.7f;
                    break;
                case WAVE:
                    waveOffset += 0.05f * animationSpeed;
                    break;
            }
            invalidate();
        });

        animator.start();
    }

    private void stopAnimation() {
        isAnimating = false;
        if (animator != null) {
            animator.cancel();
            animator = null;
        }
    }

    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);

        if (!isAnimating) return;

        float centerX = getWidth() / 2f;
        float centerY = getHeight() / 2f;

        switch (spinnerType) {
            case ORBITAL:
                drawOrbitalSpinner(canvas, centerX, centerY);
                break;
            case PULSE:
                drawPulseSpinner(canvas, centerX, centerY);
                break;
            case WAVE:
                drawWaveSpinner(canvas, centerX, centerY);
                break;
        }
    }

    private void drawOrbitalSpinner(Canvas canvas, float centerX, float centerY) {
        // Draw background circle
        canvas.drawCircle(centerX, centerY, orbitRadius, circlePaint);

        // Draw orbiting dots
        for (int i = 0; i < dotCount; i++) {
            float angle = (float) (2 * Math.PI * i / dotCount) + rotation;
            float x = centerX + (float) Math.cos(angle) * orbitRadius;
            float y = centerY + (float) Math.sin(angle) * orbitRadius;

            // Calculate dot opacity based on position (front dots are brighter)
            float progress = (float) (angle % (2 * Math.PI)) / (float) (2 * Math.PI);
            float opacity = Math.max(0.2f, Math.min(1f, progress));

            // Draw glow
            glowPaint.setAlpha((int) (opacity * 100));
            canvas.drawCircle(x, y, dotRadius * 2, glowPaint);

            // Draw dot
            dotPaint.setAlpha((int) (opacity * 255));
            canvas.drawCircle(x, y, dotRadius, dotPaint);
        }
    }

    private void drawPulseSpinner(Canvas canvas, float centerX, float centerY) {
        float radius = orbitRadius * pulseScale;

        // Draw multiple pulsing circles
        for (int i = 0; i < 3; i++) {
            float delay = i * 0.3f;
            float scale = (float) Math.sin((pulseScale - delay) * Math.PI) * 0.5f + 0.5f;
            if (scale > 0) {
                float r = radius * (1 + i * 0.3f) * scale;
                int alpha = (int) ((1f - i * 0.3f) * 150 * scale);
                circlePaint.setAlpha(alpha);
                canvas.drawCircle(centerX, centerY, r, circlePaint);
            }
        }

        // Draw center dot
        dotPaint.setAlpha(255);
        canvas.drawCircle(centerX, centerY, dotRadius * 1.5f, dotPaint);
    }

    private void drawWaveSpinner(Canvas canvas, float centerX, float centerY) {
        // Draw wave effect using dashed circle
        float[] intervals = new float[dotCount * 2];
        for (int i = 0; i < intervals.length; i++) {
            intervals[i] = (i % 2 == 0) ? 10f : 5f;
        }

        PathEffect pathEffect = new DashPathEffect(intervals, waveOffset * 50);
        circlePaint.setPathEffect(pathEffect);
        circlePaint.setAlpha(200);
        canvas.drawCircle(centerX, centerY, orbitRadius, circlePaint);
        circlePaint.setPathEffect(null);

        // Draw wave dots
        for (int i = 0; i < dotCount; i++) {
            float angle = (float) (2 * Math.PI * i / dotCount);
            float waveHeight = (float) Math.sin(waveOffset * 2 + i * 0.5f) * 10f;
            float r = orbitRadius + waveHeight;
            float x = centerX + (float) Math.cos(angle) * r;
            float y = centerY + (float) Math.sin(angle) * r;

            float opacity = (float) Math.sin(waveOffset + i * 0.3f) * 0.5f + 0.5f;
            dotPaint.setAlpha((int) (opacity * 255));
            canvas.drawCircle(x, y, dotRadius * 0.8f, dotPaint);
        }
    }

    @Override
    protected void onDetachedFromWindow() {
        super.onDetachedFromWindow();
        stopAnimation();
    }
}
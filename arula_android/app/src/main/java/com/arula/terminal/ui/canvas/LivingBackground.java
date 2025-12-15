package com.arula.terminal.ui.canvas;

import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.PorterDuff;
import android.graphics.PorterDuffXfermode;
import android.util.AttributeSet;
import android.view.View;
import androidx.annotation.Nullable;

import com.arula.terminal.R;

import java.util.ArrayList;
import java.util.List;
import java.util.Random;

/**
 * Living background with animated particles and grid effect
 * Replicates the desktop version's living background
 */
public class LivingBackground extends View {
    private List<Particle> particles;
    private List<GridLine> gridLines;
    private Paint backgroundPaint;
    private Paint particlePaint;
    private Paint gridPaint;
    private Paint glowPaint;

    private float swayAngle = 0f;
    private float travel = 0f;
    private float opacity = 1f;
    private boolean isEnabled = true;

    private int backgroundColor;
    private int accentColor;
    private int glowColor;

    private Random random = new Random();
    private ValueAnimator animator;

    public LivingBackground(Context context) {
        super(context);
        init();
    }

    public LivingBackground(Context context, @Nullable AttributeSet attrs) {
        super(context, attrs);
        init();
    }

    public LivingBackground(Context context, @Nullable AttributeSet attrs, int defStyleAttr) {
        super(context, attrs, defStyleAttr);
        init();
    }

    private void init() {
        backgroundColor = getContext().getColor(R.color.neon_background);
        accentColor = getContext().getColor(R.color.neon_accent);
        glowColor = getContext().getColor(R.color.neon_glow);

        // Initialize paints
        backgroundPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
        backgroundPaint.setColor(backgroundColor);

        particlePaint = new Paint(Paint.ANTI_ALIAS_FLAG);
        particlePaint.setStyle(Paint.Style.FILL);

        gridPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
        gridPaint.setStyle(Paint.Style.STROKE);
        gridPaint.setStrokeWidth(1f);
        gridPaint.setColor(accentColor);
        gridPaint.setAlpha(30);

        glowPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
        glowPaint.setStyle(Paint.Style.FILL);
        glowPaint.setXfermode(new PorterDuffXfermode(PorterDuff.Mode.ADD));

        particles = new ArrayList<>();
        gridLines = new ArrayList<>();

        // Start animation
        startAnimation();
    }

    @Override
    protected void onSizeChanged(int w, int h, int oldw, int oldh) {
        super.onSizeChanged(w, h, oldw, oldh);
        initializeParticles();
        initializeGrid();
    }

    private void initializeParticles() {
        particles.clear();
        int particleCount = 50;

        for (int i = 0; i < particleCount; i++) {
            Particle particle = new Particle(
                random.nextFloat() * getWidth(),
                random.nextFloat() * getHeight(),
                random.nextFloat() * 3f + 1f,
                random.nextFloat() * 0.5f + 0.5f
            );
            particles.add(particle);
        }
    }

    private void initializeGrid() {
        gridLines.clear();
        int spacing = 100;
        int numLines = 20;

        // Create horizontal grid lines
        for (int i = 0; i < numLines; i++) {
            float depth = 1f + (i * 100f);
            GridLine line = new GridLine(depth);
            gridLines.add(line);
        }
    }

    private void startAnimation() {
        animator = ValueAnimator.ofFloat(0f, 1f);
        animator.setDuration(16); // ~60fps
        animator.setRepeatCount(ValueAnimator.INFINITE);

        animator.addUpdateListener(animation -> {
            updateAnimation();
            invalidate();
        });

        animator.start();
    }

    private void updateAnimation() {
        if (!isEnabled) return;

        // Update sway angle
        swayAngle += 0.01f;

        // Update travel
        travel += 2f;
        if (travel > 1000f) {
            travel = 0f;
        }

        // Update particles
        for (Particle particle : particles) {
            particle.update(getWidth(), getHeight());
        }
    }

    public void setOpacity(float opacity) {
        this.opacity = Math.max(0f, Math.min(1f, opacity));
        updateColors();
    }

    public void setEnabled(boolean enabled) {
        this.isEnabled = enabled;
        updateColors();
    }

    private void updateColors() {
        if (isEnabled) {
            backgroundPaint.setColor(backgroundColor);
        } else {
            // Interpolate to gray when disabled
            int gray = Color.rgb(25, 25, 25);
            backgroundPaint.setColor(blendColors(backgroundColor, gray, opacity));
        }
    }

    private int blendColors(int color1, int color2, float ratio) {
        float inverseRatio = 1f - ratio;

        int r = (int) (Color.red(color1) * ratio + Color.red(color2) * inverseRatio);
        int g = (int) (Color.green(color1) * ratio + Color.green(color2) * inverseRatio);
        int b = (int) (Color.blue(color1) * ratio + Color.blue(color2) * inverseRatio);

        return Color.rgb(r, g, b);
    }

    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);

        // Draw background
        canvas.drawRect(0, 0, getWidth(), getHeight(), backgroundPaint);

        if (!isEnabled || opacity <= 0.01f) return;

        // Save canvas state for particle blending
        int saved = canvas.saveLayer(0, 0, getWidth(), getHeight(), null, Canvas.ALL_SAVE_FLAG);

        // Draw particles with glow effect
        for (Particle particle : particles) {
            float alpha = particle.alpha * opacity;

            // Draw glow
            glowPaint.setColor(accentColor);
            glowPaint.setAlpha((int) (alpha * 50));
            canvas.drawCircle(particle.x, particle.y, particle.size * 3, glowPaint);

            // Draw particle
            particlePaint.setColor(accentColor);
            particlePaint.setAlpha((int) (alpha * 255));
            canvas.drawCircle(particle.x, particle.y, particle.size, particlePaint);
        }

        // Restore canvas
        canvas.restoreToCount(saved);

        // Draw grid lines
        if (opacity > 0.5f) {
            drawGrid(canvas);
        }
    }

    private void drawGrid(Canvas canvas) {
        float centerX = getWidth() / 2f;
        float centerY = getHeight() / 2f;

        // Apply rotation based on sway angle
        canvas.save();
        canvas.rotate((float) Math.toDegrees(swayAngle), centerX, centerY);

        for (GridLine line : gridLines) {
            // Apply perspective effect
            float scale = 250f / (line.depth + 250f);
            float alpha = Math.max(0f, Math.min(1f, scale)) * opacity * 0.3f;

            gridPaint.setAlpha((int) (alpha * 255));

            // Draw horizontal grid line
            float y = centerY + (line.depth - travel % 1000f) * scale;
            Path path = new Path();
            path.moveTo(0, y);
            path.lineTo(getWidth(), y);
            canvas.drawPath(path, gridPaint);
        }

        canvas.restore();
    }

    @Override
    protected void onDetachedFromWindow() {
        super.onDetachedFromWindow();
        if (animator != null) {
            animator.cancel();
        }
    }

    /**
     * Particle class for background animation
     */
    private static class Particle {
        float x, y;
        float size;
        float alpha;
        float vx, vy;
        float life;

        Particle(float x, float y, float size, float alpha) {
            this.x = x;
            this.y = y;
            this.size = size;
            this.alpha = alpha;
            this.vx = (float) (Math.random() - 0.5) * 0.5f;
            this.vy = (float) (Math.random() - 0.5) * 0.5f;
            this.life = 1f;
        }

        void update(float width, float height) {
            // Update position
            x += vx;
            y += vy;

            // Update life
            life -= 0.002f;
            if (life <= 0) {
                // Respawn particle
                x = (float) Math.random() * width;
                y = (float) Math.random() * height;
                life = 1f;
                alpha = (float) Math.random() * 0.5f + 0.5f;
            }

            // Wrap around edges
            if (x < 0) x = width;
            if (x > width) x = 0;
            if (y < 0) y = height;
            if (y > height) y = 0;

            // Pulsing alpha
            alpha = Math.max(0.1f, Math.min(1f, alpha + (float) (Math.random() - 0.5) * 0.02f));
        }
    }

    /**
     * Grid line class for perspective effect
     */
    private static class GridLine {
        float depth;

        GridLine(float depth) {
            this.depth = depth;
        }
    }
}
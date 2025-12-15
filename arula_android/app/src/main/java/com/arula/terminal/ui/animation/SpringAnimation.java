package com.arula.terminal.ui.animation;

/**
 * Spring physics animation system for smooth, natural transitions
 * Replicates the desktop version's spring-based animations
 */
public class SpringAnimation {
    private float position;
    private float velocity;
    private float target;
    private float stiffness;
    private float damping;
    private float threshold = 0.001f;
    private boolean isAnimating = false;

    private long lastUpdateTime;
    private SpringListener listener;

    public interface SpringListener {
        void onAnimationUpdate(float position, float velocity);
        void onAnimationComplete();
    }

    public SpringAnimation() {
        this(150.0f, 0.8f); // Default stiffness and damping
    }

    public SpringAnimation(float stiffness, float damping) {
        this.position = 0.0f;
        this.velocity = 0.0f;
        this.target = 0.0f;
        this.stiffness = stiffness;
        this.damping = damping;
        this.lastUpdateTime = System.nanoTime();
    }

    /**
     * Sets the target value for the spring to animate towards
     */
    public void setTarget(float target) {
        this.target = Math.max(0.0f, Math.min(1.0f, target));
        if (Math.abs(this.target - position) > threshold) {
            isAnimating = true;
            lastUpdateTime = System.nanoTime();
        }
    }

    /**
     * Sets the target value with optional initial velocity
     */
    public void setTarget(float target, float initialVelocity) {
        this.velocity = initialVelocity;
        setTarget(target);
    }

    /**
     * Updates the spring physics simulation
     * Returns true if still animating
     */
    public boolean update() {
        if (!isAnimating) return false;

        long currentTime = System.nanoTime();
        float deltaTime = Math.min((currentTime - lastUpdateTime) / 1_000_000_000.0f, 0.016f); // Cap at 60fps
        lastUpdateTime = currentTime;

        // Spring force: F = -k * x (where x is displacement from target)
        float force = (target - position) * stiffness;

        // Update velocity with force and apply damping
        velocity = (velocity + force * deltaTime) * damping;

        // Update position
        position += velocity * deltaTime;

        // Clamp position to valid range
        position = Math.max(0.0f, Math.min(1.0f, position));

        // Check if animation is complete
        float distance = Math.abs(target - position);
        if (distance < threshold && Math.abs(velocity) < threshold) {
            position = target;
            velocity = 0.0f;
            isAnimating = false;

            if (listener != null) {
                listener.onAnimationComplete();
            }
            return false;
        }

        // Notify listener of update
        if (listener != null) {
            listener.onAnimationUpdate(position, velocity);
        }

        return true;
    }

    /**
     * Instantly sets position without animation
     */
    public void setPosition(float position) {
        this.position = Math.max(0.0f, Math.min(1.0f, position));
        this.velocity = 0.0f;
        if (Math.abs(this.target - this.position) < threshold) {
            isAnimating = false;
        }
    }

    /**
     * Applies an impulse to the spring
     */
    public void applyImpulse(float impulse) {
        velocity += impulse;
        if (!isAnimating && Math.abs(velocity) > threshold) {
            isAnimating = true;
            lastUpdateTime = System.nanoTime();
        }
    }

    // Getters and setters
    public float getPosition() { return position; }
    public float getVelocity() { return velocity; }
    public float getTarget() { return target; }
    public boolean isAnimating() { return isAnimating; }
    public boolean isOpen() { return target > 0.5f; }

    public void setStiffness(float stiffness) { this.stiffness = stiffness; }
    public void setDamping(float damping) { this.damping = damping; }
    public void setThreshold(float threshold) { this.threshold = threshold; }

    public void setListener(SpringListener listener) { this.listener = listener; }
}
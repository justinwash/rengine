use rengine::Vec2;
use std::f32::consts::PI;

use crate::track_visuals::TRACK_SCALE;

/// Bicycle-model physics with speed-dependent lateral grip.
/// Ported from the Godot CarPhysics + TractionCircle classes.
pub struct CarPhysics {
    // Tunables
    pub wheelbase: f32,
    pub mass: f32,
    pub drag_coefficient: f32,
    pub rolling_resistance: f32,
    pub max_engine_force: f32,
    pub max_brake_force: f32,
    pub max_steer_angle: f32, // radians
    pub max_speed: f32,
    pub downforce_coefficient: f32,
    pub max_angular_change_rate: f32,
    pub max_angular_velocity: f32,
    pub steering_boost_multiplier: f32,
    pub steering_boost_threshold: f32,
    pub lateral_damping_low: f32,
    pub lateral_damping_high: f32,
}

impl Default for CarPhysics {
    fn default() -> Self {
        Self {
            wheelbase: 60.0 * TRACK_SCALE,
            mass: 750.0,
            drag_coefficient: 0.04 / TRACK_SCALE,
            rolling_resistance: 1000.0 * TRACK_SCALE,
            max_engine_force: 50000.0 * TRACK_SCALE,
            max_brake_force: 120000.0 * TRACK_SCALE,
            max_steer_angle: 35.0f32.to_radians(),
            max_speed: 500.0 * TRACK_SCALE,
            downforce_coefficient: 0.5,
            max_angular_change_rate: 20.0,
            max_angular_velocity: 5.0,
            steering_boost_multiplier: 2.0,
            steering_boost_threshold: 0.8,
            lateral_damping_low: 0.88,
            lateral_damping_high: 0.65,
        }
    }
}

/// Per-frame physics result.
pub struct PhysicsResult {
    pub velocity: Vec2,
    pub angular_velocity: f32,
}

impl CarPhysics {
    /// Step the physics for one frame given driver inputs.
    ///
    /// `throttle`: 0..1, `brake`: 0..1, `steer`: -1..1
    /// `velocity`: current velocity in world space
    /// `angular_velocity`: current angular velocity (rad/s)
    /// `rotation`: current heading (radians)
    /// `instability`: 0..1, extra grip reduction from collisions etc.
    /// `dt`: delta time (seconds)
    pub fn step(
        &self,
        throttle: f32,
        brake: f32,
        steer: f32,
        velocity: Vec2,
        angular_velocity: f32,
        rotation: f32,
        instability: f32,
        dt: f32,
    ) -> PhysicsResult {
        let speed = velocity.length();

        // Longitudinal forces
        let engine_force = if throttle > 0.0 {
            throttle * self.max_engine_force
        } else {
            0.0
        };
        let brake_force = brake * self.max_brake_force;
        let drag_force = self.drag_coefficient * speed * speed;
        let resistance = if speed > 0.1 {
            self.rolling_resistance
        } else {
            0.0
        };

        let net_long_force = engine_force - brake_force - drag_force - resistance;

        // Forward direction
        let (sin_r, cos_r) = rotation.sin_cos();
        let forward = Vec2::new(cos_r, sin_r);
        let lateral = Vec2::new(-sin_r, cos_r);

        // Acceleration
        let acceleration = net_long_force / self.mass;
        let mut new_vel = velocity + forward * acceleration * dt;

        // Speed-dependent lateral damping: less grip at higher speeds
        let speed_ratio = (speed / self.max_speed).clamp(0.0, 1.0);
        let base_damping = self.lateral_damping_low
            + (self.lateral_damping_high - self.lateral_damping_low) * speed_ratio;
        // Instability further reduces grip (from collisions, off-track, etc.)
        let effective_damping = (base_damping * (1.0 - instability * 0.5)).max(0.15);

        let forward_vel = new_vel.dot(forward);
        let lateral_vel = new_vel.dot(lateral);
        let damped_lateral = lateral_vel * (1.0 - effective_damping);
        new_vel = forward * forward_vel + lateral * damped_lateral;

        // Steering (bicycle model)
        let mut new_angular_vel;
        if speed > 10.0 * TRACK_SCALE {
            let speed_factor = ((1.0 - (speed - 600.0 * TRACK_SCALE) / (400.0 * TRACK_SCALE))
                .clamp(0.4, 1.0)) as f32;
            let effective_steer = steer * self.max_steer_angle * speed_factor;
            let turn_radius = self.wheelbase / effective_steer.tan().abs().max(0.001);
            let target_angular_vel = (speed / turn_radius) * effective_steer.signum();

            let boosted = if steer.abs() > self.steering_boost_threshold {
                target_angular_vel * self.steering_boost_multiplier
            } else {
                target_angular_vel
            };

            let max_change = self.max_angular_change_rate * dt;
            let delta_ang = (boosted - angular_velocity).clamp(-max_change, max_change);
            new_angular_vel = (angular_velocity + delta_ang)
                .clamp(-self.max_angular_velocity, self.max_angular_velocity);
        } else {
            // Stronger angular damping at low speed for quicker spin recovery
            new_angular_vel = angular_velocity * 0.7;
        }

        // Spin recovery: when car heading diverges significantly from velocity,
        // gradually realign heading toward velocity direction
        if speed > 5.0 * TRACK_SCALE {
            let vel_angle = new_vel.y.atan2(new_vel.x);
            let heading_error = wrap_angle(vel_angle - rotation);
            // If heading error > 30 degrees, apply corrective angular velocity
            if heading_error.abs() > 0.5 {
                let correction = heading_error * 2.0 * dt;
                new_angular_vel += correction;
            }
        }

        // Clamp speed
        if new_vel.length() > self.max_speed {
            new_vel = new_vel.normalize() * self.max_speed;
        }

        // Prevent reversing when stopped — but snap rotation to velocity direction
        // so the car can accelerate forward again after a spin
        if new_vel.dot(forward) < 0.0 && speed < 5.0 * TRACK_SCALE {
            new_vel = Vec2::ZERO;
            new_angular_vel = 0.0;
        }

        PhysicsResult {
            velocity: new_vel,
            angular_velocity: new_angular_vel,
        }
    }
}

fn wrap_angle(a: f32) -> f32 {
    let mut a = a % (2.0 * PI);
    if a > PI {
        a -= 2.0 * PI;
    }
    if a < -PI {
        a += 2.0 * PI;
    }
    a
}

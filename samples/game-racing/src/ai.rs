use rengine::Vec2;
use std::f32::consts::PI;

use crate::driver::DriverProfile;
use crate::track::{wrap_offset, Track};
use crate::track_visuals::TRACK_SCALE;

/// AI outputs for a single frame.
pub struct AiInputs {
    pub throttle: f32, // 0..1
    pub brake: f32,    // 0..1
    pub steer: f32,    // -1..1
}

/// Racing-line-following AI. Ported from Godot RacingLineAI.
pub struct RacingAi {
    // Tuning
    pub lookahead_distance: f32,
    pub tangent_sample_distance: f32,

    // Steering zones
    pub zone1_dist: f32,
    pub zone2_dist: f32,
    pub zone3_dist: f32,
    pub zone1_gain: f32,
    pub zone2_gain: f32,
    pub zone3_gain: f32,
    pub zone4_gain: f32,
    pub zone1_damping: f32,
    pub zone2_damping: f32,
    pub zone3_damping: f32,
    pub zone4_damping: f32,

    // Feedforward
    pub feedforward_gain: f32,
    pub feedforward_lookahead: f32,

    // Steering smoothing
    pub steering_smoothing: f32,
    last_steering: f32,

    // Predictive braking
    pub predictive_lookahead: f32,
    pub tight_corner_threshold: f32,
    pub corner_speed_multiplier: f32,

    // Throttle/brake
    pub brake_divisor: f32,
    pub throttle_divisor: f32,
    pub low_speed_threshold: f32,
    pub low_speed_throttle: f32,

    // Misc
    pub max_lateral_accel: f32,
    pub min_corner_speed: f32,
    pub max_corner_speed: f32,
}

impl Default for RacingAi {
    fn default() -> Self {
        Self {
            lookahead_distance: 100.0 * TRACK_SCALE,
            tangent_sample_distance: 40.0 * TRACK_SCALE,
            zone1_dist: 8.0 * TRACK_SCALE,
            zone2_dist: 40.0 * TRACK_SCALE,
            zone3_dist: 200.0 * TRACK_SCALE,
            zone1_gain: 3.0,
            zone2_gain: 5.0,
            zone3_gain: 8.0,
            zone4_gain: 12.0,
            zone1_damping: 1.5,
            zone2_damping: 1.2,
            zone3_damping: 0.8,
            zone4_damping: 0.4,
            feedforward_gain: 8.0 * TRACK_SCALE,
            feedforward_lookahead: 60.0 * TRACK_SCALE,
            steering_smoothing: 0.55,
            last_steering: 0.0,
            predictive_lookahead: 140.0 * TRACK_SCALE,
            tight_corner_threshold: 0.006 / TRACK_SCALE,
            corner_speed_multiplier: 0.45,
            brake_divisor: 12.5 * TRACK_SCALE,
            throttle_divisor: 80.0 * TRACK_SCALE,
            low_speed_threshold: 30.0 * TRACK_SCALE,
            low_speed_throttle: 0.7,
            max_lateral_accel: 175.0 * TRACK_SCALE,
            min_corner_speed: 40.0 * TRACK_SCALE,
            max_corner_speed: 250.0 * TRACK_SCALE,
        }
    }
}

impl RacingAi {
    /// Compute steering, throttle, brake for this frame.
    pub fn compute(
        &mut self,
        track: &Track,
        car_pos: Vec2,
        car_rotation: f32,
        angular_velocity: f32,
        speed: f32,
        driver: &DriverProfile,
        speed_multiplier: f32,
        lateral_offset: f32,
        boundary_dist: f32,
        boundary_to_center: Vec2,
        nearby_cars: &[(Vec2, f32)], // (position, speed) of very close cars
    ) -> AiInputs {
        let mut steer = self.get_steering(
            track,
            car_pos,
            car_rotation,
            angular_velocity,
            lateral_offset,
        );

        // Track boundary steering correction
        let boundary_margin = 30.0 * TRACK_SCALE;
        if boundary_dist < boundary_margin {
            let urgency = (1.0 - boundary_dist / boundary_margin).clamp(0.0, 1.0);
            let correction_angle = boundary_to_center.y.atan2(boundary_to_center.x);
            let angle_diff = wrap_angle(correction_angle - car_rotation);
            let boundary_steer = (angle_diff * 2.0 * urgency).clamp(-1.0, 1.0);
            steer = lerp(steer, boundary_steer, urgency * 0.7);
        }

        // Nearby car avoidance — nudge steering away from very close cars
        for &(other_pos, _other_speed) in nearby_cars {
            let to_other = other_pos - car_pos;
            let dist = to_other.length();
            if dist < 40.0 * TRACK_SCALE && dist > 1.0 {
                let avoidance_strength =
                    ((40.0 * TRACK_SCALE - dist) / (40.0 * TRACK_SCALE)).clamp(0.0, 0.3);
                let lateral_dir = Vec2::new(-car_rotation.sin(), car_rotation.cos());
                let side = to_other.dot(lateral_dir);
                // Steer away from the nearby car
                if side > 0.0 {
                    steer -= avoidance_strength;
                } else {
                    steer += avoidance_strength;
                }
            }
        }
        steer = steer.clamp(-1.0, 1.0);

        let throttle_brake = self.get_throttle(
            track,
            car_pos,
            speed,
            driver,
            speed_multiplier,
            boundary_dist,
        );

        if throttle_brake >= 0.0 {
            AiInputs {
                throttle: throttle_brake,
                brake: 0.0,
                steer,
            }
        } else {
            AiInputs {
                throttle: 0.0,
                brake: -throttle_brake,
                steer,
            }
        }
    }

    /// Steering in [-1, 1]. Zone-based with feedforward.
    pub fn get_steering(
        &mut self,
        track: &Track,
        car_pos: Vec2,
        car_rotation: f32,
        angular_velocity: f32,
        lateral_offset: f32,
    ) -> f32 {
        let closest_offset = track.closest_offset(car_pos);
        let closest_point = offset_point(track, closest_offset, lateral_offset);
        let dist = car_pos.distance(closest_point);

        // Tangent alignment
        let p1 = track.sample(closest_offset);
        let ahead_offset = wrap_offset(closest_offset + self.tangent_sample_distance, track.length);
        let p2 = track.sample(ahead_offset);
        let line_angle = (p2 - p1).y.atan2((p2 - p1).x);
        let tangent_diff = wrap_angle(line_angle - car_rotation);

        // Pursuit target
        let target_offset = wrap_offset(closest_offset + self.lookahead_distance, track.length);
        let target_point = offset_point(track, target_offset, lateral_offset);
        let to_target = target_point - car_pos;
        let pursuit_angle = to_target.y.atan2(to_target.x);
        let pursuit_diff = wrap_angle(pursuit_angle - car_rotation);

        // Feedforward curvature
        let ff_offset = wrap_offset(closest_offset + self.feedforward_lookahead, track.length);
        let ff_curvature = track.curvature_at_offset(ff_offset);
        let ff_p0 = track.sample(wrap_offset(ff_offset - 15.0 * TRACK_SCALE, track.length));
        let ff_p1 = track.sample(ff_offset);
        let ff_p2 = track.sample(wrap_offset(ff_offset + 15.0 * TRACK_SCALE, track.length));
        let ff_v1 = (ff_p1 - ff_p0).normalize_or_zero();
        let ff_v2 = (ff_p2 - ff_p1).normalize_or_zero();
        let ff_cross = ff_v1.x * ff_v2.y - ff_v1.y * ff_v2.x;
        let ff_steering = ff_curvature * self.feedforward_gain * ff_cross.signum() * 30.0;

        // Lateral correction
        let to_line_angle = (closest_point - car_pos)
            .y
            .atan2((closest_point - car_pos).x);
        let to_line_diff = wrap_angle(to_line_angle - car_rotation);
        let correction_strength = (dist / self.zone2_dist).clamp(0.0, 1.0) * 0.35;

        // Zone selection
        let (angle_diff, gain, damping, ff_blend);
        if dist < self.zone1_dist {
            angle_diff = tangent_diff + to_line_diff * correction_strength;
            gain = self.zone1_gain;
            damping = self.zone1_damping;
            ff_blend = 1.0;
        } else if dist < self.zone2_dist {
            let blend = ((dist - self.zone1_dist) / (self.zone2_dist - self.zone1_dist)).sqrt();
            angle_diff =
                lerp(tangent_diff, pursuit_diff, blend) + to_line_diff * correction_strength * 1.5;
            gain = self.zone2_gain;
            damping = self.zone2_damping;
            ff_blend = 1.0 - blend;
        } else if dist < self.zone3_dist {
            angle_diff = pursuit_diff;
            gain = self.zone3_gain;
            damping = self.zone3_damping;
            ff_blend = 0.0;
        } else {
            angle_diff = pursuit_diff;
            gain = self.zone4_gain;
            damping = self.zone4_damping;
            ff_blend = 0.0;
        };

        let raw = (angle_diff * gain + ff_steering * ff_blend - angular_velocity * damping)
            .clamp(-1.0, 1.0);

        let smoothed = lerp(raw, self.last_steering, self.steering_smoothing).clamp(-1.0, 1.0);
        self.last_steering = smoothed;
        smoothed
    }

    /// Throttle/brake in [-1, 1]. Positive = throttle, negative = brake.
    pub fn get_throttle(
        &self,
        track: &Track,
        car_pos: Vec2,
        speed: f32,
        driver: &DriverProfile,
        speed_multiplier: f32,
        boundary_dist: f32,
    ) -> f32 {
        let closest_offset = track.closest_offset(car_pos);
        let mut target_speed = track.target_speed_at_offset(closest_offset)
            * driver.speed_multiplier()
            * speed_multiplier;
        target_speed = target_speed.clamp(self.min_corner_speed, self.max_corner_speed * 1.15);

        // Predictive braking — scan ahead for tight corners
        if speed > 50.0 * TRACK_SCALE {
            let effective_lookahead = self.predictive_lookahead / driver.brake_aggression;
            let scan_steps = 8;
            let step_size = effective_lookahead / scan_steps as f32;
            let mut worst_curvature: f32 = 0.0;

            for i in 1..=scan_steps {
                let scan_offset = wrap_offset(closest_offset + step_size * i as f32, track.length);
                worst_curvature = worst_curvature.max(track.curvature_at_offset(scan_offset));
            }

            if worst_curvature > self.tight_corner_threshold {
                let corner_limit = (self.max_lateral_accel / worst_curvature.max(0.0005)).sqrt();
                let corner_limit = corner_limit.clamp(self.min_corner_speed, self.max_corner_speed);
                target_speed = target_speed.min(corner_limit * self.corner_speed_multiplier);
            }
        }

        // Off-line penalty
        let dist = car_pos.distance(track.sample(closest_offset));
        if dist > 100.0 * TRACK_SCALE {
            let factor =
                (1.0 - (dist - 100.0 * TRACK_SCALE) / (1500.0 * TRACK_SCALE)).clamp(0.8, 1.0);
            target_speed *= factor;
        }

        // Track boundary speed reduction — slow down near edges
        let boundary_margin = 30.0 * TRACK_SCALE;
        if boundary_dist < boundary_margin {
            let urgency = (1.0 - boundary_dist / boundary_margin).clamp(0.0, 1.0);
            target_speed *= 1.0 - urgency * 0.5; // up to 50% speed reduction at the edge
        }

        // Decision
        let speed_error = target_speed - speed;

        if speed < self.low_speed_threshold {
            return (self.low_speed_throttle * driver.throttle_aggression).clamp(0.0, 1.0);
        }

        // When very close to boundary, always brake if above target
        if boundary_dist < boundary_margin * 0.5 && speed_error < 0.0 {
            return (speed_error / (self.brake_divisor * 0.5)).clamp(-1.0, 0.0);
        }

        if speed_error < 0.0 {
            (speed_error / self.brake_divisor).clamp(-1.0, 0.0)
        } else if speed_error > 3.0 * TRACK_SCALE {
            (speed_error / self.throttle_divisor * driver.throttle_aggression).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Overtake manager — handles side-by-side racing and passing maneuvers.
/// Tracks nearby cars and computes a lateral offset + speed adjustment.
pub struct OvertakeManager {
    pub state: OvertakeState,
    pub lateral_offset: f32,
    pub speed_boost: f32,
    state_timer: f32,
    car_ahead_gap: f32,
    car_ahead_speed: f32,
    car_alongside: bool,
    alongside_side: f32, // -1 = left, +1 = right
}

#[derive(Clone, Copy, PartialEq)]
pub enum OvertakeState {
    Following,
    Closing,
    Alongside,
    Committing,
    Completing,
}

impl OvertakeManager {
    pub fn new() -> Self {
        Self {
            state: OvertakeState::Following,
            lateral_offset: 0.0,
            speed_boost: 1.0,
            state_timer: 0.0,
            car_ahead_gap: f32::MAX,
            car_ahead_speed: 0.0,
            car_alongside: false,
            alongside_side: 0.0,
        }
    }

    /// Update the overtake state machine.
    pub fn update(
        &mut self,
        track: &Track,
        my_pos: Vec2,
        my_speed: f32,
        my_offset: f32,
        cars: &[(Vec2, f32, f32)], // (position, speed, track_offset) of all other cars
        driver: &DriverProfile,
        dt: f32,
    ) {
        self.state_timer += dt;

        // Find the car closest ahead on the racing line
        self.car_ahead_gap = f32::MAX;
        self.car_ahead_speed = 0.0;
        self.car_alongside = false;

        let my_off = my_offset;
        let normal = track.normal_at(my_offset);

        for &(pos, spd, off) in cars {
            let gap = wrap_offset(off - my_off, track.length);
            // Car ahead
            if gap > 5.0 * TRACK_SCALE && gap < 400.0 * TRACK_SCALE && gap < self.car_ahead_gap {
                self.car_ahead_gap = gap;
                self.car_ahead_speed = spd;
            }
            // Car alongside: similar track offset but close laterally
            let abs_gap = gap.min(track.length - gap); // handle wrap-around
            if abs_gap < 40.0 * TRACK_SCALE {
                let lateral_dist = (pos - my_pos).dot(normal);
                let longitudinal_dist = my_pos.distance(pos);
                if longitudinal_dist < 50.0 * TRACK_SCALE && lateral_dist.abs() > 8.0 * TRACK_SCALE
                {
                    self.car_alongside = true;
                    self.alongside_side = if lateral_dist > 0.0 { 1.0 } else { -1.0 };
                }
            }
        }

        let curvature = track.curvature_at_offset(my_offset);
        let in_corner = curvature > 0.003 / TRACK_SCALE;

        match self.state {
            OvertakeState::Following => {
                self.lateral_offset *= 0.95; // Decay toward center
                self.speed_boost = 1.0;

                // If a car is alongside, hold our line
                if self.car_alongside {
                    self.state = OvertakeState::Alongside;
                    self.state_timer = 0.0;
                    // Move away from the alongside car
                    self.lateral_offset = -self.alongside_side * 18.0 * TRACK_SCALE;
                }

                // Transition to Closing when gaining on car ahead
                if self.car_ahead_gap < 200.0 * TRACK_SCALE
                    && my_speed > self.car_ahead_speed + 5.0 * TRACK_SCALE
                    && driver.overtake_aggression > 0.6
                {
                    self.state = OvertakeState::Closing;
                    self.state_timer = 0.0;
                }
            }
            OvertakeState::Closing => {
                self.speed_boost = 1.02;

                if self.car_ahead_gap < 80.0 * TRACK_SCALE && !in_corner {
                    // Pick a side (negative = left, positive = right)
                    let to_car_ahead = track
                        .sample(wrap_offset(my_offset + self.car_ahead_gap, track.length))
                        - my_pos;
                    let side = if to_car_ahead.dot(normal) > 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                    self.lateral_offset = side * 22.0 * TRACK_SCALE;
                    self.state = OvertakeState::Committing;
                    self.state_timer = 0.0;
                } else if self.car_ahead_gap > 250.0 * TRACK_SCALE || self.state_timer > 6.0 {
                    self.state = OvertakeState::Following;
                    self.state_timer = 0.0;
                }
            }
            OvertakeState::Alongside => {
                // Side-by-side racing: maintain offset, match speed roughly
                self.lateral_offset = -self.alongside_side * 18.0 * TRACK_SCALE;
                self.speed_boost = 1.03; // Slight push to complete the pass

                if !self.car_alongside || self.state_timer > 5.0 {
                    // No longer alongside — either passed or dropped back
                    self.state = OvertakeState::Completing;
                    self.state_timer = 0.0;
                }
                // Abort if tight corner while alongside
                if in_corner && curvature > 0.008 / TRACK_SCALE {
                    self.lateral_offset *= 0.8;
                }
            }
            OvertakeState::Committing => {
                self.speed_boost = 1.05;

                // Transition to alongside if we detect a car next to us
                if self.car_alongside {
                    self.state = OvertakeState::Alongside;
                    self.state_timer = 0.0;
                }

                // Abort if entering a tight corner
                if in_corner && self.state_timer > 0.5 {
                    self.state = OvertakeState::Following;
                    self.lateral_offset = 0.0;
                    self.state_timer = 0.0;
                } else if self.car_ahead_gap > 60.0 * TRACK_SCALE || self.state_timer > 4.0 {
                    // We've passed or timed out
                    self.state = OvertakeState::Completing;
                    self.state_timer = 0.0;
                }
            }
            OvertakeState::Completing => {
                self.lateral_offset *= 0.92; // Merge back
                self.speed_boost = 1.02;
                if self.lateral_offset.abs() < 2.0 * TRACK_SCALE || self.state_timer > 3.0 {
                    self.lateral_offset = 0.0;
                    self.speed_boost = 1.0;
                    self.state = OvertakeState::Following;
                    self.state_timer = 0.0;
                }
            }
        }
    }

    /// Get speed multiplier for the proximity/draft situation.
    pub fn proximity_speed_limit(&self, my_speed: f32) -> f32 {
        if self.car_ahead_gap < 60.0 * TRACK_SCALE && self.state == OvertakeState::Following {
            // Don't ram the car ahead — match their speed
            self.car_ahead_speed.min(my_speed)
        } else {
            f32::MAX
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn offset_point(track: &Track, offset: f32, lateral_offset: f32) -> Vec2 {
    let center = track.sample(offset);
    if lateral_offset.abs() < 0.1 {
        center
    } else {
        center + track.normal_at(offset) * lateral_offset
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

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

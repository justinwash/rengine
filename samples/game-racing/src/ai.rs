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

// ─── Racing AI ──────────────────────────────────────────────────────────────
//
// Architecture (v2 — improved from telemetry analysis):
//
//   1. **Pre-computed speed profile** on the Track handles braking zones via a
//      backward deceleration pass.  The AI reads the target speed at the current
//      (and a short look-ahead) offset.
//
//   2. **Pure-pursuit steering** chooses a target point on the racing line at a
//      speed-proportional look-ahead distance, then computes the front-wheel
//      angle.  When far from the racing line, a **recovery steering** component
//      blends in to steer directly toward the closest line point.
//
//   3. **Speed controller** with:
//      - P-controller against the speed profile minimum in look-ahead window
//      - **Racing-line distance penalty**: target speed is reduced proportional
//        to how far the car has drifted from the racing line
//      - **Drift-rate emergency brake**: if the car is drifting further from the
//        line each frame, additional braking is applied
//      - **Trail-braking**: brake gain is boosted when steering hard, so cars
//        can brake-and-turn into corners like real F1
//      - **Boundary braking** starts earlier and is more aggressive
//
//   4. **Boundary + recovery steering** adds urgency steering when the car
//      drifts toward the track edge.
//
//   5. **Driver personality** modulates cornering speed, brake point, throttle
//      aggression, and steering smoothness per-driver.
// ────────────────────────────────────────────────────────────────────────────

pub struct RacingAi {
    // Pure-pursuit
    pub min_lookahead: f32,
    pub max_lookahead: f32,
    pub lookahead_time: f32,
    pub wheelbase: f32,

    // Speed controller
    pub brake_gain: f32,
    pub throttle_gain: f32,
    pub speed_lookahead: f32,
    pub speed_lookahead_steps: usize,

    // Boundary / racing-line awareness
    pub boundary_margin: f32,
    pub racing_line_penalty_start: f32, // start reducing speed when this far from line
    pub racing_line_penalty_full: f32,  // full speed reduction at this distance

    // State
    last_steer: f32,
    pub steering_smoothing: f32,
    prev_racing_line_dist: f32, // for drift-rate detection
}

impl Default for RacingAi {
    fn default() -> Self {
        Self {
            min_lookahead: 30.0 * TRACK_SCALE,
            max_lookahead: 200.0 * TRACK_SCALE,
            lookahead_time: 0.8,
            wheelbase: 60.0 * TRACK_SCALE,
            brake_gain: 0.07 / TRACK_SCALE,
            throttle_gain: 0.012 / TRACK_SCALE,
            speed_lookahead: 150.0 * TRACK_SCALE,
            speed_lookahead_steps: 8,
            boundary_margin: 35.0 * TRACK_SCALE,
            racing_line_penalty_start: 30.0 * TRACK_SCALE,
            racing_line_penalty_full: 80.0 * TRACK_SCALE,
            last_steer: 0.0,
            steering_smoothing: 0.15,
            prev_racing_line_dist: 0.0,
        }
    }
}

impl RacingAi {
    /// Main entry point — compute throttle, brake, steer for one frame.
    pub fn compute(
        &mut self,
        track: &Track,
        car_pos: Vec2,
        car_rotation: f32,
        _angular_velocity: f32,
        speed: f32,
        driver: &DriverProfile,
        speed_multiplier: f32,
        lateral_offset: f32,
        boundary_dist: f32,
        boundary_to_center: Vec2,
        nearby_cars: &[(Vec2, f32)],
        racing_line_dist: f32,
    ) -> AiInputs {
        // Track drift rate for emergency braking
        let drift_rate = racing_line_dist - self.prev_racing_line_dist;
        self.prev_racing_line_dist = racing_line_dist;

        // ── Steering ────────────────────────────────────────────────────
        let steer = self.compute_steering(
            track,
            car_pos,
            car_rotation,
            speed,
            lateral_offset,
            boundary_dist,
            boundary_to_center,
            nearby_cars,
            racing_line_dist,
        );

        // ── Throttle / Brake ────────────────────────────────────────────
        let tb = self.compute_throttle_brake(
            track,
            car_pos,
            speed,
            driver,
            speed_multiplier,
            boundary_dist,
            racing_line_dist,
            drift_rate,
            steer,
        );

        if tb >= 0.0 {
            AiInputs {
                throttle: tb,
                brake: 0.0,
                steer,
            }
        } else {
            AiInputs {
                throttle: 0.0,
                brake: (-tb).min(1.0),
                steer,
            }
        }
    }

    // ── Steering (pure pursuit + recovery + boundary correction) ──────────
    fn compute_steering(
        &mut self,
        track: &Track,
        car_pos: Vec2,
        car_rotation: f32,
        speed: f32,
        lateral_offset: f32,
        boundary_dist: f32,
        boundary_to_center: Vec2,
        nearby_cars: &[(Vec2, f32)],
        racing_line_dist: f32,
    ) -> f32 {
        let max_steer_angle = 35.0f32.to_radians();

        // Speed-proportional look-ahead distance
        let ld = (speed * self.lookahead_time).clamp(self.min_lookahead, self.max_lookahead);

        // Find the target point on the (potentially offset) racing line
        let my_offset = track.closest_offset(car_pos);
        let target_offset = wrap_offset(my_offset + ld, track.length);
        let target_pos = {
            let center = track.sample(target_offset);
            if lateral_offset.abs() > 0.1 {
                center + track.normal_at(target_offset) * lateral_offset
            } else {
                center
            }
        };

        // Pure pursuit: compute angle to target point in car-local frame
        let to_target = target_pos - car_pos;
        let target_angle = to_target.y.atan2(to_target.x);
        let alpha = wrap_angle(target_angle - car_rotation);

        // Classic pure pursuit: δ = atan(2·L·sin(α) / l_d)
        let ld_actual = to_target.length().max(1.0);
        let raw_steer = (2.0 * self.wheelbase * alpha.sin() / ld_actual).atan();
        let mut steer = (raw_steer / max_steer_angle).clamp(-1.0, 1.0);

        // ── Recovery steering: when far from the racing line, blend in a
        // direct-to-line component so the car doesn't slowly drift forever ───
        let recovery_start = self.racing_line_penalty_start;
        if racing_line_dist > recovery_start {
            let recovery_urgency = ((racing_line_dist - recovery_start)
                / (self.racing_line_penalty_full - recovery_start))
                .clamp(0.0, 1.0);
            // Steer directly toward the nearest racing line point
            let nearest_line_pos = track.sample(my_offset);
            let to_line = nearest_line_pos - car_pos;
            let line_angle = to_line.y.atan2(to_line.x);
            let line_diff = wrap_angle(line_angle - car_rotation);
            let recovery_steer = (line_diff / max_steer_angle).clamp(-1.0, 1.0);
            // Squared urgency for progressive blend
            let blend = recovery_urgency * recovery_urgency * 0.5;
            steer = lerp(steer, recovery_steer, blend);
        }

        // ── Boundary correction ─────────────────────────────────────────
        if boundary_dist < self.boundary_margin {
            let urgency = (1.0 - boundary_dist / self.boundary_margin).powi(2);
            let correction_angle = boundary_to_center.y.atan2(boundary_to_center.x);
            let angle_diff = wrap_angle(correction_angle - car_rotation);
            let boundary_steer = (angle_diff / max_steer_angle).clamp(-1.0, 1.0);
            steer = lerp(steer, boundary_steer, urgency * 0.85);
        }

        // ── Nearby car avoidance ────────────────────────────────────────
        for &(other_pos, _other_speed) in nearby_cars {
            let to_other = other_pos - car_pos;
            let dist = to_other.length();
            if dist < 40.0 * TRACK_SCALE && dist > 1.0 {
                let strength =
                    ((40.0 * TRACK_SCALE - dist) / (40.0 * TRACK_SCALE)).clamp(0.0, 0.25);
                let lateral_dir = Vec2::new(-car_rotation.sin(), car_rotation.cos());
                let side = to_other.dot(lateral_dir);
                if side > 0.0 {
                    steer -= strength;
                } else {
                    steer += strength;
                }
            }
        }

        steer = steer.clamp(-1.0, 1.0);

        // Smoothing — reduced from 0.30 to 0.12 for snappier steering response
        let smoothed = lerp(steer, self.last_steer, self.steering_smoothing);
        self.last_steer = smoothed;
        smoothed.clamp(-1.0, 1.0)
    }

    // ── Throttle / Brake ────────────────────────────────────────────────
    //
    // Enhanced with:
    //  - Racing-line distance penalty (slow when drifting off line)
    //  - Drift-rate emergency brake (lit up when drifting AWAY from line)
    //  - Trail-braking (higher brake gain when steering hard)
    //  - Earlier, more aggressive boundary braking
    fn compute_throttle_brake(
        &self,
        track: &Track,
        car_pos: Vec2,
        speed: f32,
        driver: &DriverProfile,
        speed_multiplier: f32,
        boundary_dist: f32,
        racing_line_dist: f32,
        drift_rate: f32,
        steer_amount: f32,
    ) -> f32 {
        let my_offset = track.closest_offset(car_pos);

        // Find the minimum target speed within the look-ahead window.
        // cornering_caution modulates how far ahead we look: cautious drivers
        // spot braking zones earlier, aggressive drivers brake late.
        let effective_lookahead = self.speed_lookahead * driver.cornering_caution;
        let step = effective_lookahead / self.speed_lookahead_steps as f32;
        let mut min_target = track.target_speed_at_offset(my_offset);
        for i in 1..=self.speed_lookahead_steps {
            let off = wrap_offset(my_offset + step * i as f32, track.length);
            min_target = min_target.min(track.target_speed_at_offset(off));
        }

        let mut target_speed = min_target * driver.speed_multiplier() * speed_multiplier;

        // ── Racing-line distance penalty ────────────────────────────────
        // The further from the racing line, the more we reduce target speed.
        // This prevents the car from accelerating while very far off-track.
        if racing_line_dist > self.racing_line_penalty_start {
            let off_line_ratio = ((racing_line_dist - self.racing_line_penalty_start)
                / (self.racing_line_penalty_full - self.racing_line_penalty_start))
                .clamp(0.0, 1.0);
            // Up to 25% speed reduction when fully off-line
            target_speed *= 1.0 - off_line_ratio * 0.25;
        }

        // ── Boundary proximity slowdown ─────────────────────────────────
        if boundary_dist < self.boundary_margin {
            let urgency = (1.0 - boundary_dist / self.boundary_margin).clamp(0.0, 1.0);
            target_speed *= 1.0 - urgency * 0.4;
        }

        let speed_error = target_speed - speed;

        // Low speed kickstart
        if speed < 30.0 * TRACK_SCALE {
            return (0.7 * driver.throttle_aggression).clamp(0.0, 1.0);
        }

        // ── Drift-rate emergency brake ──────────────────────────────────
        // If we're drifting AWAY from the racing line (drift_rate > 0) and
        // already far off-line, apply emergency braking.
        if drift_rate > 0.5 && racing_line_dist > self.racing_line_penalty_start * 1.2 {
            let drift_severity = (drift_rate / 2.0).clamp(0.0, 1.0);
            let off_severity = ((racing_line_dist - self.racing_line_penalty_start)
                / (self.racing_line_penalty_full * 0.5))
                .clamp(0.0, 1.0);
            let emergency_brake = drift_severity * off_severity * 0.6;
            if emergency_brake > 0.15 {
                return -emergency_brake.min(1.0);
            }
        }

        // ── Hard brake near boundary ────────────────────────────────────
        if boundary_dist < self.boundary_margin * 0.4 && speed_error < 0.0 {
            return (speed_error * self.brake_gain * 2.0).clamp(-1.0, 0.0);
        }

        if speed_error < 0.0 {
            // ── Trail-braking: boost brake gain when steering hard ──────
            // Real cars brake into corners (trail-braking). Increase brake
            // gain proportionally to how much the driver is turning.
            let steer_factor = 1.0 + steer_amount.abs() * 0.6;
            (speed_error * self.brake_gain * driver.brake_aggression * steer_factor)
                .clamp(-1.0, 0.0)
        } else if speed_error > 2.0 * TRACK_SCALE {
            // Only accelerate hard if NOT drifting significantly off-line
            let throttle_limit = if racing_line_dist > self.racing_line_penalty_start {
                let off_ratio = ((racing_line_dist - self.racing_line_penalty_start)
                    / (self.racing_line_penalty_full - self.racing_line_penalty_start))
                    .clamp(0.0, 1.0);
                1.0 - off_ratio * 0.5
            } else {
                1.0
            };
            (speed_error * self.throttle_gain * driver.throttle_aggression)
                .clamp(0.0, 1.0)
                .min(throttle_limit)
        } else {
            // In the sweet spot — coast
            0.0
        }
    }

    /// Simplified steering for launch phase.
    pub fn get_steering_simple(
        &mut self,
        track: &Track,
        car_pos: Vec2,
        car_rotation: f32,
        _angular_velocity: f32,
    ) -> f32 {
        let ld = self.min_lookahead * 1.5;
        let my_offset = track.closest_offset(car_pos);
        let target_offset = wrap_offset(my_offset + ld, track.length);
        let target_pos = track.sample(target_offset);

        let to_target = target_pos - car_pos;
        let target_angle = to_target.y.atan2(to_target.x);
        let alpha = wrap_angle(target_angle - car_rotation);
        let ld_actual = to_target.length().max(1.0);
        let raw_steer = (2.0 * self.wheelbase * alpha.sin() / ld_actual).atan();
        let max_steer_angle = 35.0f32.to_radians();
        (raw_steer / max_steer_angle).clamp(-1.0, 1.0)
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

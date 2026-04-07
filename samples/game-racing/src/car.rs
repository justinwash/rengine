use rengine::{Color, TextureId, Vec2};
use std::f32::consts::PI;

use crate::ai::{OvertakeManager, RacingAi};
use crate::driver::DriverProfile;
use crate::physics::CarPhysics;
use crate::track::{wrap_offset, Track};
use crate::track_visuals::TRACK_SCALE;

/// Per-car state.
pub struct Car {
    // Identity
    pub index: usize,
    pub driver: DriverProfile,
    pub body_color: Color,
    pub tex: TextureId,

    // Transform
    pub pos: Vec2,
    pub rotation: f32, // radians, 0 = right

    // Physics
    pub velocity: Vec2,
    pub angular_velocity: f32,
    pub speed: f32,
    pub physics: CarPhysics,

    // AI
    pub ai: RacingAi,
    pub overtake: OvertakeManager,

    // State
    pub state: CarState,
    pub launch_timer: f32,
    pub launch_reaction_delay: f32,
    pub launch_throttle_cap: f32,

    // Race tracking
    pub current_lap: u32,
    pub current_lap_time: f32,
    pub last_lap_time: f32,
    pub best_lap_time: f32,
    pub race_time: f32,
    pub race_progress: f32, // laps + fraction
    pub place: usize,
    pub start_place: usize,
    pub last_offset: f32,
    pub initialized_racing: bool,

    // Current AI outputs (for UI)
    pub ai_throttle: f32,
    pub ai_brake: f32,
    pub ai_steer: f32,

    // Comeback / draft
    pub draft_amount: f32,
    pub finished: bool,

    // Instability from collisions/off-track (0 = stable, 1 = fully unsettled)
    pub instability: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CarState {
    Grid,
    Launching,
    Racing,
}

/// Predefined team colors.
const TEAM_COLORS: &[Color] = &[
    Color::new(0.9, 0.1, 0.1, 1.0), // Red
    Color::new(0.0, 0.8, 0.9, 1.0), // Cyan
    Color::new(1.0, 0.6, 0.0, 1.0), // Orange
    Color::new(0.1, 0.1, 0.9, 1.0), // Blue
    Color::new(0.0, 0.7, 0.2, 1.0), // Green
    Color::new(1.0, 1.0, 0.0, 1.0), // Yellow
    Color::new(0.8, 0.0, 0.8, 1.0), // Magenta
    Color::new(0.2, 0.2, 0.2, 1.0), // Dark grey
    Color::new(1.0, 0.4, 0.7, 1.0), // Pink
    Color::new(0.6, 0.4, 0.2, 1.0), // Brown
];

impl Car {
    pub fn new(index: usize, pos: Vec2, rotation: f32, tex: TextureId) -> Self {
        let driver = DriverProfile::preset(index);
        let body_color = TEAM_COLORS[index % TEAM_COLORS.len()];
        Self {
            index,
            driver,
            body_color,
            tex,
            pos,
            rotation,
            velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            speed: 0.0,
            physics: CarPhysics::default(),
            ai: RacingAi::default(),
            overtake: OvertakeManager::new(),
            state: CarState::Grid,
            launch_timer: 0.0,
            launch_reaction_delay: 0.0,
            launch_throttle_cap: 0.0,
            current_lap: 0,
            current_lap_time: 0.0,
            last_lap_time: 0.0,
            best_lap_time: f32::MAX,
            race_time: 0.0,
            race_progress: 0.0,
            place: index + 1,
            start_place: index + 1,
            last_offset: 0.0,
            initialized_racing: false,
            ai_throttle: 0.0,
            ai_brake: 0.0,
            ai_steer: 0.0,
            draft_amount: 0.0,
            finished: false,
            instability: 0.0,
        }
    }

    /// Roll launch quality based on driver skill + randomness.
    pub fn roll_launch(&mut self) {
        // Use a simple hash for deterministic-ish randomness
        let hash = simple_hash(self.index as u64 ^ 0xDEAD_BEEF);
        let roll = (self.driver.launch_skill + (hash as f32 / u64::MAX as f32) * 0.5 - 0.25)
            .clamp(0.0, 1.0);

        if roll >= 0.85 {
            self.launch_reaction_delay = 0.02;
            self.launch_throttle_cap = 0.95;
        } else if roll >= 0.65 {
            self.launch_reaction_delay = 0.1;
            self.launch_throttle_cap = 0.85;
        } else if roll >= 0.40 {
            self.launch_reaction_delay = 0.22;
            self.launch_throttle_cap = 0.75;
        } else if roll >= 0.20 {
            self.launch_reaction_delay = 0.4;
            self.launch_throttle_cap = 0.60;
        } else {
            self.launch_reaction_delay = 0.7;
            self.launch_throttle_cap = 0.45;
        }
        self.launch_timer = 0.0;
    }

    /// Process one frame of simulation.
    pub fn update(&mut self, track: &Track, other_cars: &[(Vec2, f32, f32)], dt: f32) {
        match self.state {
            CarState::Grid => {
                // Idle on grid, rev engine
                self.speed = 0.0;
            }
            CarState::Launching => {
                self.process_launch(track, dt);
            }
            CarState::Racing => {
                self.process_racing(track, other_cars, dt);
            }
        }
    }

    fn process_launch(&mut self, track: &Track, dt: f32) {
        self.launch_timer += dt;
        self.race_time += dt;

        if self.launch_timer < self.launch_reaction_delay {
            // Waiting...
            self.speed = 0.0;
            return;
        }

        // Accelerating with limited throttle, aiming straight ahead
        let time_since_go = self.launch_timer - self.launch_reaction_delay;
        let throttle_ramp = (time_since_go / 2.0).clamp(0.0, 1.0);
        let effective_throttle = lerp_f32(self.launch_throttle_cap, 1.0, throttle_ramp);

        // Simple: apply physics with current heading, no fancy steering
        let steer = {
            // Gently steer toward the racing line once up to speed
            if self.speed > 60.0 * TRACK_SCALE {
                self.ai
                    .get_steering_simple(track, self.pos, self.rotation, self.angular_velocity)
            } else {
                0.0
            }
        };

        let result = self.physics.step(
            effective_throttle,
            0.0,
            steer,
            self.velocity,
            self.angular_velocity,
            self.rotation,
            0.0,
            dt,
        );

        self.velocity = result.velocity;
        self.angular_velocity = result.angular_velocity;
        self.rotation += self.angular_velocity * dt;
        self.pos += self.velocity * dt;
        self.speed = self.velocity.length();

        self.ai_throttle = effective_throttle;
        self.ai_brake = 0.0;
        self.ai_steer = steer;

        self.update_lap(track);

        // Transition to racing once near target speed
        let target = track.target_speed_at_offset(track.closest_offset(self.pos));
        if self.speed > target * 0.6 {
            self.state = CarState::Racing;
        }
    }

    fn process_racing(&mut self, track: &Track, other_cars: &[(Vec2, f32, f32)], dt: f32) {
        self.driver.update_variation(dt);
        self.race_time += dt;

        // Decay instability
        self.instability = (self.instability - dt * 2.5).max(0.0);

        let my_offset = track.closest_offset(self.pos);

        // Track boundary info
        let (boundary_dist, boundary_to_center) = track.boundary_info(self.pos);

        // Off-track instability boost
        if boundary_dist < 10.0 * TRACK_SCALE {
            self.instability = (self.instability + dt * 2.0).min(0.6);
        }

        // Update overtake state machine
        self.overtake.update(
            track,
            self.pos,
            self.speed,
            my_offset,
            other_cars,
            &self.driver,
            dt,
        );

        let lat_offset = self.overtake.lateral_offset;
        let mut speed_mult = self.overtake.speed_boost;

        // Comeback: trailing cars get a small boost
        let comeback = 1.0 + ((self.place as f32 - 1.0) * 0.006).min(0.035);
        speed_mult *= comeback;

        // Draft: speed boost when close behind another car
        self.update_draft(other_cars, track, my_offset, dt);
        speed_mult *= 1.0 + self.draft_amount * 0.10;

        // Build nearby cars list for avoidance
        let nearby_cars: Vec<(Vec2, f32)> = other_cars
            .iter()
            .filter(|&&(pos, _, _)| pos.distance(self.pos) < 50.0 * TRACK_SCALE)
            .map(|&(pos, spd, _)| (pos, spd))
            .collect();

        // Distance from the racing line (for line-distance-aware speed control)
        let racing_line_dist = track.racing_line_dist(self.pos);

        // Compute AI inputs
        let inputs = self.ai.compute(
            track,
            self.pos,
            self.rotation,
            self.angular_velocity,
            self.speed,
            &self.driver,
            speed_mult,
            lat_offset,
            boundary_dist,
            boundary_to_center,
            &nearby_cars,
            racing_line_dist,
        );

        let mut throttle = inputs.throttle;
        let mut brake = inputs.brake;
        let steer = inputs.steer;

        // Proximity: don't ram the car ahead
        let prox_limit = self.overtake.proximity_speed_limit(self.speed);
        if prox_limit < f32::MAX && self.speed > prox_limit {
            let overspeed = self.speed - prox_limit;
            if overspeed < 20.0 * TRACK_SCALE {
                let reduction = (overspeed / (25.0 * TRACK_SCALE)).clamp(0.0, 1.0);
                throttle *= 1.0 - reduction;
            } else {
                let prox_brake =
                    ((overspeed - 15.0 * TRACK_SCALE) / (300.0 * TRACK_SCALE)).clamp(0.0, 0.25);
                brake = brake.max(prox_brake);
                throttle *= (1.0 - prox_brake * 1.5).clamp(0.0, 1.0);
            }
        }

        // Physics step
        let result = self.physics.step(
            throttle,
            brake,
            steer,
            self.velocity,
            self.angular_velocity,
            self.rotation,
            self.instability,
            dt,
        );

        self.velocity = result.velocity;
        self.angular_velocity = result.angular_velocity;
        self.rotation += self.angular_velocity * dt;
        self.pos += self.velocity * dt;
        self.speed = self.velocity.length();

        // Recovery: when nearly stopped after a spin, snap heading toward racing line tangent
        if self.speed < 5.0 * TRACK_SCALE {
            let tangent = track.tangent_at(my_offset);
            let target_rot = tangent.y.atan2(tangent.x);
            let diff = wrap_angle_car(target_rot - self.rotation);
            self.rotation += diff * (dt * 3.0).min(1.0);
        }

        self.ai_throttle = throttle;
        self.ai_brake = brake;
        self.ai_steer = steer;

        self.current_lap_time += dt;
        self.update_lap(track);
    }

    fn update_draft(
        &mut self,
        others: &[(Vec2, f32, f32)],
        track: &Track,
        my_offset: f32,
        dt: f32,
    ) {
        let curvature = track.curvature_at_offset(my_offset);
        if curvature > 0.003 / TRACK_SCALE {
            // No draft in corners
            self.draft_amount = (self.draft_amount - dt * 2.0).max(0.0);
            return;
        }

        let mut closest_ahead_dist = f32::MAX;
        for &(pos, _spd, off) in others {
            let gap = wrap_offset(off - my_offset, track.length);
            if gap > 5.0 * TRACK_SCALE && gap < 450.0 * TRACK_SCALE {
                let dist = self.pos.distance(pos);
                closest_ahead_dist = closest_ahead_dist.min(dist);
            }
        }

        if closest_ahead_dist < 450.0 * TRACK_SCALE {
            self.draft_amount = (self.draft_amount + dt / 1.0).min(1.0);
        } else {
            self.draft_amount = (self.draft_amount - dt * 2.0).max(0.0);
        }
    }

    fn update_lap(&mut self, track: &Track) {
        let current_offset = track.closest_offset(self.pos);
        let fraction = current_offset / track.length;

        // Initialize lap tracking
        if self.current_lap == 0
            && (self.state == CarState::Racing || self.state == CarState::Launching)
        {
            self.last_offset = current_offset;
            self.current_lap = 1;
            self.current_lap_time = 0.0;
            self.initialized_racing = true;
            return;
        }

        // Detect start/finish crossing
        let diff = current_offset - self.last_offset;
        if diff.abs() > track.length * 0.5 {
            if !self.initialized_racing {
                if diff < 0.0 {
                    // Crossed forward
                    self.last_lap_time = self.current_lap_time;
                    if self.current_lap_time < self.best_lap_time && self.current_lap_time > 1.0 {
                        self.best_lap_time = self.current_lap_time;
                    }
                    self.current_lap += 1;
                    self.current_lap_time = 0.0;
                }
            } else {
                self.initialized_racing = false;
            }
        }

        self.last_offset = current_offset;

        // Race progress
        if self.current_lap >= 1 {
            self.race_progress = (self.current_lap - 1) as f32 + fraction;
        }
    }
}

fn simple_hash(mut x: u64) -> u64 {
    x = x.wrapping_mul(0x517cc1b727220a95);
    x ^= x >> 32;
    x = x.wrapping_mul(0x6c62272e07bb0142);
    x ^= x >> 32;
    x
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn wrap_angle_car(a: f32) -> f32 {
    let mut a = a % (2.0 * PI);
    if a > PI {
        a -= 2.0 * PI;
    }
    if a < -PI {
        a += 2.0 * PI;
    }
    a
}

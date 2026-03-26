use rengine::circle_overlap;

use crate::car::{Car, CarState};
use crate::track::Track;

const CAR_COLLISION_RADIUS: f32 = 15.0;

/// Top-level race state.
pub struct Race {
    pub total_laps: u32,
    pub countdown: f32, // seconds remaining in countdown, 0 = race started
    pub started: bool,
    pub finished: bool,
    pub race_time: f32,
    pub final_order: Vec<usize>, // car indices in finishing order
}

impl Race {
    pub fn new(total_laps: u32) -> Self {
        Self {
            total_laps,
            countdown: 4.0, // 3-2-1-GO
            started: false,
            finished: false,
            race_time: 0.0,
            final_order: Vec::new(),
        }
    }

    /// Main race update. Mutates cars and race state.
    pub fn update(&mut self, cars: &mut [Car], track: &Track, dt: f32) {
        // Countdown
        if !self.started {
            self.countdown -= dt;
            if self.countdown <= 0.0 {
                self.started = true;
                self.countdown = 0.0;
                // Start all cars
                for car in cars.iter_mut() {
                    if car.state == CarState::Grid {
                        car.roll_launch();
                        car.state = CarState::Launching;
                    }
                }
            }
            return;
        }

        self.race_time += dt;

        // Build the "other cars" array for AI lookups
        // Each car needs to see all other cars: (pos, speed, track_offset)
        let car_data: Vec<(rengine::Vec2, f32, f32)> = cars
            .iter()
            .map(|c| (c.pos, c.speed, track.closest_offset(c.pos)))
            .collect();

        // Update each car
        for i in 0..cars.len() {
            if cars[i].finished {
                continue;
            }

            // Build other-cars list excluding self
            let others: Vec<(rengine::Vec2, f32, f32)> = car_data
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, d)| *d)
                .collect();

            cars[i].update(track, &others, dt);

            // Check for race finish
            if cars[i].current_lap > self.total_laps && !cars[i].finished {
                cars[i].finished = true;
                self.final_order.push(i);
            }
        }

        // --- Car-car collision resolution ---
        resolve_collisions(cars);

        // Update standings (sort by race progress)
        let mut indices: Vec<usize> = (0..cars.len()).collect();
        indices.sort_by(|&a, &b| {
            // Finished cars first (by finishing order)
            let a_fin = cars[a].finished;
            let b_fin = cars[b].finished;
            if a_fin && !b_fin {
                return std::cmp::Ordering::Less;
            }
            if !a_fin && b_fin {
                return std::cmp::Ordering::Greater;
            }
            if a_fin && b_fin {
                let a_ord = self.final_order.iter().position(|&x| x == a).unwrap_or(999);
                let b_ord = self.final_order.iter().position(|&x| x == b).unwrap_or(999);
                return a_ord.cmp(&b_ord);
            }
            // Both racing: sort by race_progress descending
            cars[b]
                .race_progress
                .partial_cmp(&cars[a].race_progress)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (place, &car_idx) in indices.iter().enumerate() {
            cars[car_idx].place = place + 1;
        }

        // Check if all cars finished
        if self.final_order.len() == cars.len() {
            self.finished = true;
        }
    }

    /// Get countdown display number (3, 2, 1, or 0 for GO).
    pub fn countdown_display(&self) -> Option<u32> {
        if self.started {
            return None;
        }
        if self.countdown > 3.0 {
            None
        } else if self.countdown > 2.0 {
            Some(3)
        } else if self.countdown > 1.0 {
            Some(2)
        } else if self.countdown > 0.0 {
            Some(1)
        } else {
            Some(0)
        }
    }
}

/// Push overlapping cars apart and exchange velocity along the collision normal.
fn resolve_collisions(cars: &mut [Car]) {
    let n = cars.len();
    for i in 0..n {
        for j in (i + 1)..n {
            if cars[i].finished || cars[j].finished {
                continue;
            }
            if let Some(mtv) = circle_overlap(
                cars[i].pos,
                CAR_COLLISION_RADIUS,
                cars[j].pos,
                CAR_COLLISION_RADIUS,
            ) {
                // Separate: push each car half the MTV
                cars[i].pos += mtv * 0.5;
                cars[j].pos -= mtv * 0.5;

                // Velocity exchange along the collision normal (partially elastic)
                let normal = mtv.normalize();
                let rel_v = cars[i].velocity - cars[j].velocity;
                let v_along = rel_v.dot(normal);

                // Only resolve if cars are moving toward each other
                if v_along < 0.0 {
                    let restitution = 0.4;
                    let impulse = normal * v_along * (1.0 + restitution) * 0.5;
                    cars[i].velocity -= impulse;
                    cars[j].velocity += impulse;

                    // Mild spin from impact — enough to unsettle but not destroy
                    let impact_severity = (v_along.abs() / 200.0).min(1.5);
                    let cross_i = normal.x * cars[i].velocity.y - normal.y * cars[i].velocity.x;
                    let cross_j = normal.x * cars[j].velocity.y - normal.y * cars[j].velocity.x;
                    cars[i].angular_velocity += cross_i.signum() * impact_severity * 0.15;
                    cars[j].angular_velocity -= cross_j.signum() * impact_severity * 0.15;

                    // Clamp angular velocity after collision to prevent death spirals
                    cars[i].angular_velocity = cars[i].angular_velocity.clamp(-3.0, 3.0);
                    cars[j].angular_velocity = cars[j].angular_velocity.clamp(-3.0, 3.0);

                    // Minor speed reduction on impact
                    let speed_loss = (impact_severity * 0.05).clamp(0.0, 0.10);
                    cars[i].velocity *= 1.0 - speed_loss;
                    cars[j].velocity *= 1.0 - speed_loss;

                    // Mild instability — unsettled but recoverable
                    let instab = (impact_severity * 0.4).clamp(0.1, 0.5);
                    cars[i].instability = (cars[i].instability + instab).min(0.7);
                    cars[j].instability = (cars[j].instability + instab).min(0.7);

                    // Update speed caches
                    cars[i].speed = cars[i].velocity.length();
                    cars[j].speed = cars[j].velocity.length();
                }
            }
        }
    }
}

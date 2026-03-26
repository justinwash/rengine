use std::fs::File;
use std::io::{BufWriter, Write};

use crate::car::Car;
use crate::track::Track;
use crate::track_visuals::TRACK_SCALE;

pub struct Telemetry {
    writer: BufWriter<File>,
    tracked_cars: Vec<usize>,
    frame: u64,
}

impl Telemetry {
    pub fn new(tracked_cars: &[usize]) -> Self {
        let file = File::create("telemetry.csv").expect("Failed to create telemetry.csv");
        let mut writer = BufWriter::new(file);
        writeln!(
            writer,
            "frame,car,driver,lap,pos_x,pos_y,speed,throttle,brake,steer,angular_vel,rotation,track_offset,lateral_dist,overtake_state,draft"
        )
        .unwrap();
        Self {
            writer,
            tracked_cars: tracked_cars.to_vec(),
            frame: 0,
        }
    }

    pub fn log_frame(&mut self, cars: &[Car], track: &Track) {
        self.frame += 1;
        // Log every 3rd frame to keep file size reasonable
        if self.frame % 3 != 0 {
            return;
        }
        for &idx in &self.tracked_cars {
            if idx >= cars.len() {
                continue;
            }
            let car = &cars[idx];
            let offset = track.closest_offset(car.pos);
            let on_line = track.sample(offset);
            let lateral_dist = car.pos.distance(on_line) / TRACK_SCALE;
            let overtake_state = match car.overtake.state {
                crate::ai::OvertakeState::Following => "follow",
                crate::ai::OvertakeState::Closing => "closing",
                crate::ai::OvertakeState::Alongside => "alongside",
                crate::ai::OvertakeState::Committing => "commit",
                crate::ai::OvertakeState::Completing => "complete",
            };
            let _ = writeln!(
                self.writer,
                "{},{},{},{},{:.1},{:.1},{:.1},{:.3},{:.3},{:.3},{:.4},{:.4},{:.1},{:.2},{},{}",
                self.frame,
                idx,
                car.driver.abbreviation,
                car.current_lap,
                car.pos.x,
                car.pos.y,
                car.speed / TRACK_SCALE,
                car.ai_throttle,
                car.ai_brake,
                car.ai_steer,
                car.angular_velocity,
                car.rotation,
                offset / TRACK_SCALE,
                lateral_dist,
                overtake_state,
                car.draft_amount,
            );
        }
    }

    pub fn flush(&mut self) {
        let _ = self.writer.flush();
    }
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        let _ = self.writer.flush();
    }
}

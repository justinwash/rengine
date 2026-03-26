use crate::state::{FpsInput, FpsSim};

pub fn bot_input(sim: &FpsSim, frame: u32, player: u32) -> FpsInput {
    let me = &sim.players[player as usize];
    let opp_idx = 1 - player as usize;
    let opp = &sim.players[opp_idx];

    if !me.alive() {
        return FpsInput::default();
    }

    let dx = opp.x - me.x;
    let dz = opp.z - me.z;
    let dy = opp.y - me.y;
    let dist_xz = (dx * dx + dz * dz).sqrt().max(0.01);

    let target_yaw = dx.atan2(-dz);
    let target_pitch = (dy / dist_xz).atan();

    let mut yaw_err = target_yaw - me.yaw;
    while yaw_err > std::f32::consts::PI {
        yaw_err -= 2.0 * std::f32::consts::PI;
    }
    while yaw_err < -std::f32::consts::PI {
        yaw_err += 2.0 * std::f32::consts::PI;
    }
    let pitch_err = target_pitch - me.pitch;

    let smoothing = 0.15_f32;
    let dyaw = yaw_err * smoothing;
    let dpitch = pitch_err * smoothing;

    let (look_dx, look_dy) = FpsInput::encode_look(dyaw, dpitch);

    let mut flags = 0u8;

    let cycle = ((frame + player * 137) % 300) as f32;

    if cycle < 120.0 {
        if dist_xz > 4.0 {
            flags |= FpsInput::FORWARD;
        }
        flags |= FpsInput::LEFT;
    } else if cycle < 180.0 {
        flags |= FpsInput::RIGHT;
        if dist_xz > 8.0 {
            flags |= FpsInput::FORWARD;
        }
    } else if cycle < 240.0 {
        if dist_xz < 10.0 {
            flags |= FpsInput::BACK;
        }
        flags |= FpsInput::RIGHT;
    } else {
        if dist_xz > 4.0 {
            flags |= FpsInput::FORWARD;
        }
        flags |= FpsInput::LEFT;
    }

    if yaw_err.abs() < 0.15 && pitch_err.abs() < 0.2 && dist_xz < 20.0 && opp.alive() {
        flags |= FpsInput::SHOOT;
    }

    if frame % 150 == (player * 67) % 150 {
        flags |= FpsInput::JUMP;
    }

    FpsInput {
        flags,
        _pad: 0,
        look_dx,
        look_dy,
        _pad2: [0; 2],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CollisionWall, FpsSim};
    use crate::PLAYER_HEIGHT;

    fn make_sim() -> FpsSim {
        let walls = vec![
            CollisionWall::new(0.0, 0.0, 28.0, 0.0),
            CollisionWall::new(0.0, 12.0, 28.0, 12.0),
            CollisionWall::new(0.0, 0.0, 0.0, 12.0),
            CollisionWall::new(28.0, 0.0, 28.0, 12.0),
        ];
        let door_defs = Vec::new();
        let spawn_points = [[4.0, PLAYER_HEIGHT, 4.0], [24.0, PLAYER_HEIGHT, 8.0]];
        FpsSim::new(walls, door_defs, spawn_points)
    }

    fn run_match(frames: u32) -> (Vec<u8>, Vec<u64>) {
        let mut sim = make_sim();
        let mut checksums = Vec::with_capacity(frames as usize);
        for f in 0..frames {
            let inputs: Vec<FpsInput> = (0..2).map(|p| bot_input(&sim, f, p)).collect();
            sim.advance(&inputs);
            let snap = sim.save();
            checksums.push(rengine::fletcher64(&snap));
        }
        (sim.save(), checksums)
    }

    const TEN_SECONDS: u32 = 600;

    #[test]
    fn determinism_two_identical_runs() {
        let (state_a, checksums_a) = run_match(TEN_SECONDS);
        let (state_b, checksums_b) = run_match(TEN_SECONDS);
        assert_eq!(checksums_a, checksums_b, "per-frame checksums diverged");
        assert_eq!(state_a, state_b, "final states differ");
    }

    #[test]
    fn save_load_determinism() {
        let (_, baseline) = run_match(TEN_SECONDS);

        let mut sim = make_sim();
        for f in 0..TEN_SECONDS {
            if f % 120 == 0 {
                let snap = sim.save();
                sim.load(&snap);
            }
            let inputs: Vec<FpsInput> = (0..2).map(|p| bot_input(&sim, f, p)).collect();
            sim.advance(&inputs);
            let snap = sim.save();
            let cs = rengine::fletcher64(&snap);
            assert_eq!(cs, baseline[f as usize], "checksum mismatch at frame {f}");
        }
    }

    #[test]
    fn rollback_packet_loss_determinism() {
        let mut all_inputs: Vec<Vec<FpsInput>> = Vec::new();
        {
            let mut sim = make_sim();
            for f in 0..TEN_SECONDS {
                let inputs: Vec<FpsInput> = (0..2).map(|p| bot_input(&sim, f, p)).collect();
                sim.advance(&inputs);
                all_inputs.push(inputs);
            }
        }

        let mut baseline_checksums = Vec::with_capacity(TEN_SECONDS as usize);
        {
            let mut sim = make_sim();
            for f in 0..TEN_SECONDS {
                sim.advance(&all_inputs[f as usize]);
                baseline_checksums.push(rengine::fletcher64(&sim.save()));
            }
        }

        const SAVE_INTERVAL: u32 = 30;
        const LOSS_START: u32 = 23;
        const LOSS_END: u32 = 28;

        let mut sim = make_sim();
        let mut last_good_snap: Vec<u8> = sim.save();
        let mut last_good_frame: u32 = 0;

        for f in 0..TEN_SECONDS {
            let phase = f % SAVE_INTERVAL;
            if phase == 0 {
                last_good_snap = sim.save();
                last_good_frame = f;
            }

            if phase >= LOSS_START && phase <= LOSS_END {
                let mut wrong = all_inputs[f as usize].clone();
                wrong[1] = FpsInput::default();
                sim.advance(&wrong);
            } else {
                sim.advance(&all_inputs[f as usize]);
            }

            if phase == LOSS_END {
                sim.load(&last_good_snap);
                for rf in last_good_frame..=f {
                    sim.advance(&all_inputs[rf as usize]);
                }
                assert_eq!(
                    rengine::fletcher64(&sim.save()),
                    baseline_checksums[f as usize],
                    "state diverged after rollback at frame {f}"
                );
            }
        }
    }
}

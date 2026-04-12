use crate::state::{FightInput, FighterData, FighterState};
use crate::{KICK_RANGE, PUNCH_RANGE};

pub fn bot_input(me: &FighterData, opponent: &FighterData, frame: u32, player: u32) -> FightInput {
    let dx = opponent.x - me.x;
    let dist = dx.abs();
    let mut flags = 0u8;

    let rng = frame
        .wrapping_mul(2654435761)
        .wrapping_add(player.wrapping_mul(1013904223));

    if dist > PUNCH_RANGE + 20.0 {
        if dx > 0.0 {
            flags |= FightInput::RIGHT;
        } else {
            flags |= FightInput::LEFT;
        }
    }

    if dist < KICK_RANGE + 10.0 && me.can_act() {
        if rng % 3 == 0 {
            flags |= FightInput::PUNCH;
        } else if rng % 3 == 1 {
            flags |= FightInput::KICK;
        }
    }

    if rng % 37 == 0 && me.is_on_ground() {
        flags |= FightInput::JUMP;
    }

    if dist < KICK_RANGE + 30.0
        && matches!(
            opponent.state,
            FighterState::Punching | FighterState::Kicking
        )
        && rng % 4 != 0
    {
        flags |= FightInput::CROUCH;
    }

    FightInput {
        flags,
        _pad: [0; 3],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::FightSim;

    const SIXTY_SECONDS: u32 = 60 * 60;

    fn bot_inputs(sim: &FightSim, frame: u32) -> [FightInput; 2] {
        [
            bot_input(&sim.p1, &sim.p2, frame, 0),
            bot_input(&sim.p2, &sim.p1, frame, 1),
        ]
    }

    fn fletcher64(data: &[u8]) -> u64 {
        let mut s1: u32 = 0;
        let mut s2: u32 = 0;
        for &b in data {
            s1 = s1.wrapping_add(b as u32);
            s2 = s2.wrapping_add(s1);
        }
        ((s2 as u64) << 32) | s1 as u64
    }

    fn run_match(frames: u32) -> (Vec<u8>, Vec<u64>) {
        let mut sim = FightSim::new();
        let mut checksums = Vec::with_capacity(frames as usize);
        for f in 0..frames {
            let inputs = bot_inputs(&sim, f);
            sim.advance(&inputs);
            checksums.push(fletcher64(&sim.save()));
        }
        (sim.save(), checksums)
    }

    #[test]
    fn determinism_two_identical_runs() {
        let (state_a, checksums_a) = run_match(SIXTY_SECONDS);
        let (state_b, checksums_b) = run_match(SIXTY_SECONDS);
        assert_eq!(checksums_a, checksums_b, "Per-frame checksums diverged");
        assert_eq!(state_a, state_b, "Final states differ");
    }

    #[test]
    fn save_load_determinism() {
        let mut sim_base = FightSim::new();
        let mut all_inputs: Vec<[FightInput; 2]> =
            Vec::with_capacity(SIXTY_SECONDS as usize);
        let mut baseline_checksums: Vec<u64> =
            Vec::with_capacity(SIXTY_SECONDS as usize);

        for f in 0..SIXTY_SECONDS {
            let inputs = bot_inputs(&sim_base, f);
            all_inputs.push(inputs);
            sim_base.advance(&inputs);
            baseline_checksums.push(fletcher64(&sim_base.save()));
        }
        let baseline_final = sim_base.save();

        let mut sim = FightSim::new();
        for f in 0..SIXTY_SECONDS {
            if f % 120 == 0 {
                let snap = sim.save();
                sim.load(&snap);
            }
            sim.advance(&all_inputs[f as usize]);
            let cksum = fletcher64(&sim.save());
            assert_eq!(
                cksum, baseline_checksums[f as usize],
                "Checksum diverged after save/load at frame {f}"
            );
        }
        assert_eq!(sim.save(), baseline_final);
    }

    #[test]
    fn rollback_packet_loss_determinism() {
        let mut sim_base = FightSim::new();
        let mut all_inputs: Vec<[FightInput; 2]> =
            Vec::with_capacity(SIXTY_SECONDS as usize);
        let mut baseline_checksums: Vec<u64> =
            Vec::with_capacity(SIXTY_SECONDS as usize);

        for f in 0..SIXTY_SECONDS {
            let inputs = bot_inputs(&sim_base, f);
            all_inputs.push(inputs);
            sim_base.advance(&inputs);
            baseline_checksums.push(fletcher64(&sim_base.save()));
        }
        let baseline_final = sim_base.save();

        const SAVE_INTERVAL: u32 = 30;
        const LOSS_START: u32 = 23;
        const LOSS_END: u32 = 28;

        let mut sim = FightSim::new();
        let mut last_good_frame: u32 = 0;
        let mut last_good_snap: Vec<u8> = sim.save();

        let mut f: u32 = 0;
        while f < SIXTY_SECONDS {
            let phase = f % SAVE_INTERVAL;

            if phase == 0 {
                last_good_snap = sim.save();
                last_good_frame = f;
            }

            if phase >= LOSS_START && phase <= LOSS_END {
                let mut wrong = all_inputs[f as usize];
                wrong[1] = FightInput::default();
                sim.advance(&wrong);
            } else {
                sim.advance(&all_inputs[f as usize]);
            }

            if phase == LOSS_END {
                sim.load(&last_good_snap);
                for rf in last_good_frame..=f {
                    sim.advance(&all_inputs[rf as usize]);
                }
                let cksum = fletcher64(&sim.save());
                assert_eq!(
                    cksum, baseline_checksums[f as usize],
                    "Rollback correction failed at frame {f}"
                );
            }

            f += 1;
        }

        assert_eq!(
            sim.save(),
            baseline_final,
            "Final states differ after packet loss simulation"
        );
    }
}

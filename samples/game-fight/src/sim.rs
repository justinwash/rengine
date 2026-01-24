use crate::state::{
    Facing, FightInput, FightSim, FightSnapshot, FighterState, Spark,
};
use crate::{
    ATTACK_DURATION, BLOCK_REDUCTION, FIGHTER_H, FIGHTER_W, FIXED_DT, GRAVITY, GROUND_Y,
    HIT_STUN, JUMP_VEL, KICK_DAMAGE, KICK_RANGE, KNOCKBACK_SPEED, MAX_HP, PUNCH_DAMAGE,
    PUNCH_RANGE, ROUND_WIN_PAUSE, STAGE_LEFT, STAGE_RIGHT, WALK_SPEED,
};

impl FightSim {
    pub fn advance(&mut self, inputs: &[FightInput]) {
        let dt = FIXED_DT;


        for spark in &mut self.sparks {
            spark.x += spark.vx * dt;
            spark.y += spark.vy * dt;
            spark.vy -= 800.0 * dt;
            spark.life -= dt;
        }
        self.sparks.retain(|s| s.life > 0.0);


        if self.round_pause > 0.0 {
            self.round_pause -= dt;
            if self.round_pause <= 0.0 {
                self.reset_round();
            }
            return;
        }

        let p1_input = inputs[0];
        let p2_input = inputs[1];

        self.update_fighter_input(true, p1_input, dt);
        self.update_fighter_input(false, p2_input, dt);

        self.apply_physics(true, dt);
        self.apply_physics(false, dt);


        if self.p1.can_act() || self.p1.state == FighterState::Jumping {
            self.p1.facing = if self.p1.x < self.p2.x {
                Facing::Right
            } else {
                Facing::Left
            };
        }
        if self.p2.can_act() || self.p2.state == FighterState::Jumping {
            self.p2.facing = if self.p2.x < self.p1.x {
                Facing::Right
            } else {
                Facing::Left
            };
        }

        self.check_attack_hit(true);
        self.check_attack_hit(false);

        self.push_apart();


        if self.p1.hp <= 0 {
            self.p2.wins += 1;
            self.round_pause = ROUND_WIN_PAUSE;
            self.round_number += 1;
        } else if self.p2.hp <= 0 {
            self.p1.wins += 1;
            self.round_pause = ROUND_WIN_PAUSE;
            self.round_number += 1;
        }
    }

    pub fn save(&self) -> Vec<u8> {
        let snapshot = FightSnapshot {
            p1: self.p1.clone(),
            p2: self.p2.clone(),
            round_pause: self.round_pause,
            round_number: self.round_number,
            sparks: self.sparks.clone(),
        };
        bincode::serialize(&snapshot).expect("serialize fight state")
    }

    pub fn load(&mut self, data: &[u8]) {
        let snapshot: FightSnapshot =
            bincode::deserialize(data).expect("deserialize fight state");
        self.p1 = snapshot.p1;
        self.p2 = snapshot.p2;
        self.round_pause = snapshot.round_pause;
        self.round_number = snapshot.round_number;
        self.sparks = snapshot.sparks;
    }

    fn update_fighter_input(&mut self, is_p1: bool, input: FightInput, dt: f32) {
        let fighter = if is_p1 { &mut self.p1 } else { &mut self.p2 };


        if fighter.state_timer > 0.0 {
            fighter.state_timer -= dt;
            if fighter.state_timer <= 0.0 {
                match fighter.state {
                    FighterState::Punching | FighterState::Kicking | FighterState::HitStun => {
                        fighter.state = FighterState::Idle;
                        fighter.vel_x = 0.0;
                    }
                    _ => {}
                }
            } else {
                return;
            }
        }


        if fighter.is_on_ground() {
            if input.punch() && fighter.can_act() {
                fighter.state = FighterState::Punching;
                fighter.state_timer = ATTACK_DURATION;
                fighter.attack_hit_connected = false;
                fighter.vel_x = 0.0;
                return;
            }
            if input.kick() && fighter.can_act() {
                fighter.state = FighterState::Kicking;
                fighter.state_timer = ATTACK_DURATION;
                fighter.attack_hit_connected = false;
                fighter.vel_x = 0.0;
                return;
            }
            if input.jump() && fighter.can_act() {
                fighter.vel_y = JUMP_VEL;
                fighter.state = FighterState::Jumping;
                fighter.y += 1.0;
                return;
            }
            if input.crouch() {
                fighter.state = FighterState::Blocking;
                fighter.vel_x = 0.0;
                return;
            }

            fighter.vel_x = 0.0;
            if input.left() {
                fighter.vel_x = -WALK_SPEED;
                fighter.state = FighterState::Walking;
            } else if input.right() {
                fighter.vel_x = WALK_SPEED;
                fighter.state = FighterState::Walking;
            } else {
                fighter.state = FighterState::Idle;
            }
        } else {
            fighter.vel_x = 0.0;
            if input.left() {
                fighter.vel_x = -WALK_SPEED * 0.6;
            } else if input.right() {
                fighter.vel_x = WALK_SPEED * 0.6;
            }
        }
    }

    fn apply_physics(&mut self, is_p1: bool, dt: f32) {
        let fighter = if is_p1 { &mut self.p1 } else { &mut self.p2 };

        fighter.vel_y += GRAVITY * dt;
        fighter.x += fighter.vel_x * dt;
        fighter.y += fighter.vel_y * dt;

        if fighter.y <= GROUND_Y {
            fighter.y = GROUND_Y;
            fighter.vel_y = 0.0;
            if fighter.state == FighterState::Jumping {
                fighter.state = FighterState::Idle;
            }
        }

        fighter.x = fighter
            .x
            .clamp(STAGE_LEFT + FIGHTER_W / 2.0, STAGE_RIGHT - FIGHTER_W / 2.0);
    }

    fn check_attack_hit(&mut self, attacker_is_p1: bool) {
        let (atk_state, atk_facing, atk_x, atk_y, atk_connected) = {
            let a = if attacker_is_p1 { &self.p1 } else { &self.p2 };
            (a.state, a.facing, a.x, a.y, a.attack_hit_connected)
        };

        if atk_connected {
            return;
        }

        let (range, damage) = match atk_state {
            FighterState::Punching => (PUNCH_RANGE, PUNCH_DAMAGE),
            FighterState::Kicking => (KICK_RANGE, KICK_DAMAGE),
            _ => return,
        };

        let attack_x = match atk_facing {
            Facing::Right => atk_x + range,
            Facing::Left => atk_x - range,
        };

        let (def_x, def_y, def_state) = {
            let d = if attacker_is_p1 { &self.p2 } else { &self.p1 };
            (d.x, d.y, d.state)
        };
        let def_left = def_x - FIGHTER_W / 2.0;
        let def_right = def_x + FIGHTER_W / 2.0;
        let def_top = def_y + FIGHTER_H;
        let atk_hit_y = atk_y + FIGHTER_H * 0.5;

        let hit_x_in = match atk_facing {
            Facing::Right => attack_x >= def_left && atk_x < def_right,
            Facing::Left => attack_x <= def_right && atk_x > def_left,
        };

        if hit_x_in && atk_hit_y <= def_top && atk_hit_y >= def_y {
            let actual_damage = if def_state == FighterState::Blocking {
                (damage as f32 * BLOCK_REDUCTION) as i32
            } else {
                damage
            };

            let kb_dir = if atk_facing == Facing::Right {
                1.0
            } else {
                -1.0
            };

            {
                let defender = if attacker_is_p1 {
                    &mut self.p2
                } else {
                    &mut self.p1
                };
                defender.hp -= actual_damage;
                if def_state != FighterState::Blocking {
                    defender.state = FighterState::HitStun;
                    defender.state_timer = HIT_STUN;
                    defender.vel_x = kb_dir * KNOCKBACK_SPEED;
                    defender.vel_y = 150.0;
                } else {
                    defender.vel_x = kb_dir * KNOCKBACK_SPEED * 0.3;
                }
            }

            {
                let attacker = if attacker_is_p1 {
                    &mut self.p1
                } else {
                    &mut self.p2
                };
                attacker.attack_hit_connected = true;
            }

            let spark_x = (atk_x + def_x) / 2.0;
            let spark_y = atk_hit_y;
            for i in 0..8 {
                let angle = (i as f32 / 8.0) * std::f32::consts::TAU;
                let speed = 200.0 + (i as f32 * 30.0);
                self.sparks.push(Spark {
                    x: spark_x,
                    y: spark_y,
                    vx: angle.cos() * speed,
                    vy: angle.sin() * speed - 100.0,
                    life: 0.3,
                });
            }
        }
    }

    fn push_apart(&mut self) {
        let min_dist = FIGHTER_W * 0.6;
        let dx = self.p2.x - self.p1.x;
        let dist = dx.abs();
        if dist < min_dist {
            let push = (min_dist - dist) / 2.0;
            if dx > 0.0 {
                self.p1.x -= push;
                self.p2.x += push;
            } else {
                self.p1.x += push;
                self.p2.x -= push;
            }
            self.p1.x = self
                .p1
                .x
                .clamp(STAGE_LEFT + FIGHTER_W / 2.0, STAGE_RIGHT - FIGHTER_W / 2.0);
            self.p2.x = self
                .p2
                .x
                .clamp(STAGE_LEFT + FIGHTER_W / 2.0, STAGE_RIGHT - FIGHTER_W / 2.0);
        }
    }

    pub fn reset_round(&mut self) {
        self.p1.x = 250.0;
        self.p1.y = GROUND_Y;
        self.p1.vel_x = 0.0;
        self.p1.vel_y = 0.0;
        self.p1.hp = MAX_HP;
        self.p1.state = FighterState::Idle;
        self.p1.state_timer = 0.0;

        self.p2.x = 550.0;
        self.p2.y = GROUND_Y;
        self.p2.vel_x = 0.0;
        self.p2.vel_y = 0.0;
        self.p2.hp = MAX_HP;
        self.p2.state = FighterState::Idle;
        self.p2.state_timer = 0.0;

        self.sparks.clear();
    }
}

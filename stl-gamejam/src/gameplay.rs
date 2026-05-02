use rengine::*;

const STARTING_SCRAP: i32 = 60;
const MAX_CONVERSION: f32 = 100.0;
const CONTRACTOR_HP: f32 = 48.0;
const CONTRACTOR_SPEED: f32 = 54.0;
const CONTRACTOR_SLOWED_SPEED: f32 = 26.0;
const CONTRACTOR_INSTALL_TIME: f32 = 1.8;
const CONTRACTOR_INSTALL_CONVERSION: f32 = 16.0;
const CONTRACTOR_SCRAP_REWARD: i32 = 12;

const EVENT_FENCE_COST: i32 = 20;
const EVENT_FENCE_MAX_HP: f32 = 90.0;
const EVENT_FENCE_CONTACT_RANGE: f32 = 18.0;
const EVENT_FENCE_DAMAGE_INTERVAL: f32 = 0.75;
const EVENT_FENCE_DAMAGE_PER_HIT: f32 = 16.0;

const POP_BOX_COST: i32 = 28;
const POP_BOX_RANGE: f32 = 42.0;
const POP_BOX_DAMAGE: f32 = 28.0;
const POP_BOX_COOLDOWN: f32 = 2.35;
const POP_BOX_SLOW_DURATION: f32 = 1.2;

const DOOR_MAX_HP: f32 = 72.0;
const DOOR_DAMAGE_INTERVAL: f32 = 0.8;
const DOOR_DAMAGE_PER_HIT: f32 = 15.0;

const BUILD_FLASH_DURATION: f32 = 0.45;
const HIT_FLASH_DURATION: f32 = 0.16;
const EXPLOSION_DURATION: f32 = 0.36;

const WORLD_LEFT: f32 = 0.0;
const WORLD_BOTTOM: f32 = -260.0;
const WORLD_WIDTH: f32 = 840.0;
const WORLD_HEIGHT: f32 = 520.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePhase {
    Build,
    Wave,
    Victory,
    Defeat,
}

#[derive(Clone, Copy)]
struct WavePlan {
    spawn_lanes: &'static [usize],
    spawn_interval: f32,
    clear_bonus: i32,
}

const WAVE_ONE: &[usize] = &[0, 1, 2, 3, 0];
const WAVE_TWO: &[usize] = &[0, 2, 1, 3, 0, 2, 1, 3];
const WAVE_THREE: &[usize] = &[0, 1, 2, 3, 0, 3, 1, 2, 0, 2];

const WAVE_PLANS: [WavePlan; 3] = [
    WavePlan {
        spawn_lanes: WAVE_ONE,
        spawn_interval: 0.95,
        clear_bonus: 12,
    },
    WavePlan {
        spawn_lanes: WAVE_TWO,
        spawn_interval: 0.84,
        clear_bonus: 18,
    },
    WavePlan {
        spawn_lanes: WAVE_THREE,
        spawn_interval: 0.74,
        clear_bonus: 24,
    },
];

#[derive(Clone, Copy)]
enum BuildSlotKind {
    Fence,
    Trap,
}

enum BuiltInstall {
    EventFence {
        hp: f32,
        build_timer: f32,
        hit_flash: f32,
    },
    BreakerPopBox {
        cooldown: f32,
        flash_timer: f32,
        build_timer: f32,
    },
}

struct BuildSlot {
    lane_id: usize,
    kind: BuildSlotKind,
    rect: Rect,
    build: Option<BuiltInstall>,
}

impl BuildSlot {
    fn fence(lane_id: usize, center: Vec2) -> Self {
        Self {
            lane_id,
            kind: BuildSlotKind::Fence,
            rect: Rect::new(center.x - 20.0, center.y - 13.0, 40.0, 26.0),
            build: None,
        }
    }

    fn trap(lane_id: usize, center: Vec2) -> Self {
        Self {
            lane_id,
            kind: BuildSlotKind::Trap,
            rect: Rect::new(center.x - 15.0, center.y - 15.0, 30.0, 30.0),
            build: None,
        }
    }

    fn cost(&self) -> i32 {
        match self.kind {
            BuildSlotKind::Fence => EVENT_FENCE_COST,
            BuildSlotKind::Trap => POP_BOX_COST,
        }
    }

    fn label(&self) -> &'static str {
        match self.kind {
            BuildSlotKind::Fence => "Event Fence",
            BuildSlotKind::Trap => "Breaker Pop Box",
        }
    }

    fn build_short_label(&self) -> &'static str {
        match self.kind {
            BuildSlotKind::Fence => "F",
            BuildSlotKind::Trap => "P",
        }
    }

    fn center(&self) -> Vec2 {
        self.rect.center()
    }

    fn is_empty(&self) -> bool {
        self.build.is_none()
    }

    fn install(&mut self) {
        self.build = Some(match self.kind {
            BuildSlotKind::Fence => BuiltInstall::EventFence {
                hp: EVENT_FENCE_MAX_HP,
                build_timer: BUILD_FLASH_DURATION,
                hit_flash: 0.0,
            },
            BuildSlotKind::Trap => BuiltInstall::BreakerPopBox {
                cooldown: 0.0,
                flash_timer: 0.0,
                build_timer: BUILD_FLASH_DURATION,
            },
        });
    }
}

struct Lane {
    name: &'static str,
    entry_label: &'static str,
    waypoints: Vec<Vec2>,
}

struct LaneDoor {
    rect: Rect,
    hp: f32,
    flash_timer: f32,
    broken: bool,
}

impl LaneDoor {
    fn new(rect: Rect) -> Self {
        Self {
            rect,
            hp: DOOR_MAX_HP,
            flash_timer: 0.0,
            broken: false,
        }
    }

    fn reset(&mut self) {
        self.hp = DOOR_MAX_HP;
        self.flash_timer = 0.0;
        self.broken = false;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnemyState {
    Walking,
    BreakingDoor,
    Installing,
}

struct Enemy {
    lane_id: usize,
    position: Vec2,
    next_waypoint: usize,
    hp: f32,
    slow_timer: f32,
    attack_timer: f32,
    install_timer: f32,
    anim_timer: f32,
    facing: Vec2,
    state: EnemyState,
}

impl Enemy {
    fn new(lane_id: usize, start: Vec2) -> Self {
        Self {
            lane_id,
            position: start,
            next_waypoint: 1,
            hp: CONTRACTOR_HP,
            slow_timer: 0.0,
            attack_timer: 0.0,
            install_timer: 0.0,
            anim_timer: 0.0,
            facing: Vec2::new(1.0, 0.0),
            state: EnemyState::Walking,
        }
    }
}

struct Explosion {
    position: Vec2,
    timer: f32,
}

struct RenderView {
    origin: Vec2,
    scale: f32,
}

impl RenderView {
    fn point(&self, point: Vec2) -> Vec2 {
        self.origin + point * self.scale
    }

    fn rect(&self, rect: Rect) -> Rect {
        Rect::new(
            self.origin.x + rect.x * self.scale,
            self.origin.y + rect.y * self.scale,
            rect.width * self.scale,
            rect.height * self.scale,
        )
    }

    fn scalar(&self, value: f32) -> f32 {
        value * self.scale
    }

    fn font_size(&self, base: f32) -> f32 {
        (base * self.scale.clamp(0.8, 1.15)).max(8.0).round()
    }
}

pub struct ArmoryDefenseGame {
    phase: GamePhase,
    scrap: i32,
    conversion: f32,
    waves_started: usize,
    pending_spawns: Vec<usize>,
    spawn_cursor: usize,
    spawn_timer: f32,
    contractors_stopped: u32,
    message: String,
    objective_rect: Rect,
    lanes: Vec<Lane>,
    doors: Vec<LaneDoor>,
    server_bays: Vec<Rect>,
    slots: Vec<BuildSlot>,
    enemies: Vec<Enemy>,
    explosions: Vec<Explosion>,
}

impl ArmoryDefenseGame {
    pub fn new() -> Self {
        Self {
            phase: GamePhase::Build,
            scrap: STARTING_SCRAP,
            conversion: 0.0,
            waves_started: 0,
            pending_spawns: Vec::new(),
            spawn_cursor: 0,
            spawn_timer: 0.0,
            contractors_stopped: 0,
            message: "Fortify the open server floor before the contractors breach the doors."
                .to_string(),
            objective_rect: Rect::new(360.0, -60.0, 120.0, 120.0),
            lanes: vec![
                Lane {
                    name: "West Freight",
                    entry_label: "West Freight",
                    waypoints: vec![
                        Vec2::new(-40.0, 130.0),
                        Vec2::new(82.0, 130.0),
                        Vec2::new(210.0, 130.0),
                        Vec2::new(315.0, 130.0),
                        Vec2::new(420.0, 0.0),
                    ],
                },
                Lane {
                    name: "South Utility",
                    entry_label: "South Utility",
                    waypoints: vec![
                        Vec2::new(220.0, -320.0),
                        Vec2::new(220.0, -180.0),
                        Vec2::new(220.0, -70.0),
                        Vec2::new(330.0, -70.0),
                        Vec2::new(420.0, 0.0),
                    ],
                },
                Lane {
                    name: "North Service",
                    entry_label: "North Service",
                    waypoints: vec![
                        Vec2::new(520.0, 320.0),
                        Vec2::new(520.0, 180.0),
                        Vec2::new(520.0, 70.0),
                        Vec2::new(450.0, 70.0),
                        Vec2::new(420.0, 0.0),
                    ],
                },
                Lane {
                    name: "East Dock",
                    entry_label: "East Dock",
                    waypoints: vec![
                        Vec2::new(880.0, -40.0),
                        Vec2::new(730.0, -40.0),
                        Vec2::new(620.0, -40.0),
                        Vec2::new(510.0, -40.0),
                        Vec2::new(420.0, 0.0),
                    ],
                },
            ],
            doors: vec![
                LaneDoor::new(Rect::new(68.0, 92.0, 28.0, 76.0)),
                LaneDoor::new(Rect::new(182.0, -193.0, 76.0, 26.0)),
                LaneDoor::new(Rect::new(482.0, 167.0, 76.0, 26.0)),
                LaneDoor::new(Rect::new(717.0, -79.0, 26.0, 78.0)),
            ],
            server_bays: vec![
                Rect::new(120.0, -200.0, 110.0, 38.0),
                Rect::new(120.0, -110.0, 110.0, 38.0),
                Rect::new(120.0, -20.0, 110.0, 38.0),
                Rect::new(120.0, 70.0, 110.0, 38.0),
                Rect::new(120.0, 160.0, 110.0, 38.0),
                Rect::new(310.0, -200.0, 110.0, 38.0),
                Rect::new(310.0, -110.0, 110.0, 38.0),
                Rect::new(310.0, 70.0, 110.0, 38.0),
                Rect::new(310.0, 160.0, 110.0, 38.0),
                Rect::new(500.0, -200.0, 110.0, 38.0),
                Rect::new(500.0, -110.0, 110.0, 38.0),
                Rect::new(500.0, -20.0, 110.0, 38.0),
                Rect::new(500.0, 70.0, 110.0, 38.0),
                Rect::new(500.0, 160.0, 110.0, 38.0),
            ],
            slots: vec![
                BuildSlot::fence(0, Vec2::new(182.0, 130.0)),
                BuildSlot::trap(0, Vec2::new(286.0, 112.0)),
                BuildSlot::fence(0, Vec2::new(348.0, 82.0)),
                BuildSlot::fence(1, Vec2::new(220.0, -112.0)),
                BuildSlot::trap(1, Vec2::new(274.0, -74.0)),
                BuildSlot::trap(1, Vec2::new(360.0, -26.0)),
                BuildSlot::fence(2, Vec2::new(520.0, 112.0)),
                BuildSlot::trap(2, Vec2::new(472.0, 88.0)),
                BuildSlot::trap(2, Vec2::new(438.0, 42.0)),
                BuildSlot::fence(3, Vec2::new(634.0, -40.0)),
                BuildSlot::trap(3, Vec2::new(556.0, -10.0)),
                BuildSlot::fence(3, Vec2::new(490.0, -30.0)),
            ],
            enemies: Vec::new(),
            explosions: Vec::new(),
        }
    }

    pub fn world_bounds() -> Rect {
        Rect::new(WORLD_LEFT, WORLD_BOTTOM, WORLD_WIDTH, WORLD_HEIGHT)
    }

    pub fn phase(&self) -> GamePhase {
        self.phase
    }

    pub fn scrap(&self) -> i32 {
        self.scrap
    }

    pub fn conversion(&self) -> f32 {
        self.conversion
    }

    pub fn contractors_stopped(&self) -> u32 {
        self.contractors_stopped
    }

    pub fn total_waves(&self) -> usize {
        WAVE_PLANS.len()
    }

    pub fn displayed_wave(&self) -> usize {
        match self.phase {
            GamePhase::Build => (self.waves_started + 1).min(WAVE_PLANS.len()),
            GamePhase::Wave | GamePhase::Victory | GamePhase::Defeat => self.waves_started.max(1),
        }
    }

    pub fn phase_label(&self) -> &'static str {
        match self.phase {
            GamePhase::Build => "Build Phase",
            GamePhase::Wave => "Wave Phase",
            GamePhase::Victory => "Victory",
            GamePhase::Defeat => "Defeat",
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn hovered_slot_index(&self, mouse: Vec2) -> Option<usize> {
        self.slot_at_point(mouse)
    }

    pub fn hover_message(&self, mouse: Vec2) -> Option<String> {
        let slot = &self.slots[self.slot_at_point(mouse)?];
        let lane_name = self.lanes[slot.lane_id].name;

        Some(match slot.build.as_ref() {
            None => format!("{lane_name}: click for {} (${})", slot.label(), slot.cost()),
            Some(BuiltInstall::EventFence { hp, .. }) => {
                format!("{lane_name}: fence {:.0}/{:.0} hp", hp, EVENT_FENCE_MAX_HP)
            }
            Some(BuiltInstall::BreakerPopBox { cooldown, .. }) if *cooldown > 0.0 => {
                format!("{lane_name}: pop box {:.1}s recharge", cooldown)
            }
            Some(BuiltInstall::BreakerPopBox { .. }) => format!("{lane_name}: pop box armed"),
        })
    }

    pub fn start_wave(&mut self) {
        if self.phase != GamePhase::Build || self.waves_started >= WAVE_PLANS.len() {
            return;
        }

        let plan = WAVE_PLANS[self.waves_started];
        self.pending_spawns = plan.spawn_lanes.to_vec();
        self.spawn_cursor = 0;
        self.spawn_timer = 0.25;
        self.waves_started += 1;
        self.phase = GamePhase::Wave;
        self.explosions.clear();
        for door in &mut self.doors {
            door.reset();
        }
        self.message = format!(
            "Wave {} live. Contractors are breaching every entry door.",
            self.waves_started
        );
    }

    pub fn handle_build_click(&mut self, mouse: Vec2) {
        if self.phase != GamePhase::Build {
            return;
        }

        for slot in &mut self.slots {
            if !slot.rect.contains_point(mouse) {
                continue;
            }

            if !slot.is_empty() {
                self.message = format!("{} pad is already fortified.", slot.label());
                return;
            }

            if self.scrap < slot.cost() {
                self.message = format!("Need {} scrap for {}.", slot.cost(), slot.label());
                return;
            }

            self.scrap -= slot.cost();
            slot.install();
            self.message = format!("Placed {} for {} scrap.", slot.label(), slot.cost());
            return;
        }
    }

    pub fn update(&mut self, engine: &Engine) {
        let dt = engine.dt();
        self.update_effects(dt);

        if self.phase != GamePhase::Wave {
            return;
        }

        self.update_spawns(dt);
        self.trigger_traps();
        self.update_enemies(dt);

        if self.conversion >= MAX_CONVERSION {
            self.conversion = MAX_CONVERSION;
            self.phase = GamePhase::Defeat;
            self.message = "Install bays went hot. The server floor is lost.".to_string();
            return;
        }

        if self.spawn_cursor >= self.pending_spawns.len() && self.enemies.is_empty() {
            let clear_bonus = WAVE_PLANS[self.waves_started - 1].clear_bonus;
            self.scrap += clear_bonus;

            if self.waves_started >= WAVE_PLANS.len() {
                self.phase = GamePhase::Victory;
                self.message =
                    "The contractor push collapsed. The Armory holds tonight.".to_string();
            } else {
                self.phase = GamePhase::Build;
                self.message = format!(
                    "Wave {} repelled. Bonus scrap +{}.",
                    self.waves_started, clear_bonus
                );
            }
        }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas,
        origin: Vec2,
        scale: f32,
        hovered_slot: Option<usize>,
        mouse: Option<Vec2>,
        show_route_overlay: bool,
        show_floor_grid: bool,
    ) {
        let view = RenderView { origin, scale };
        self.draw_floor(canvas, &view, show_route_overlay, show_floor_grid);
        self.draw_objective(canvas, &view);
        self.draw_doors(canvas, &view);
        self.draw_slots(canvas, &view, hovered_slot);
        self.draw_enemies(canvas, &view);
        self.draw_explosions(canvas, &view);

        if let Some(mouse) = mouse.filter(|mouse| Self::world_bounds().contains_point(*mouse)) {
            self.draw_cursor(canvas, &view, mouse);
        }
    }

    fn update_effects(&mut self, dt: f32) {
        for door in &mut self.doors {
            door.flash_timer = (door.flash_timer - dt).max(0.0);
        }

        for slot in &mut self.slots {
            match slot.build.as_mut() {
                Some(BuiltInstall::EventFence {
                    hp,
                    build_timer,
                    hit_flash,
                }) => {
                    *build_timer = (*build_timer - dt).max(0.0);
                    *hit_flash = (*hit_flash - dt).max(0.0);
                    if *hp <= 0.0 {
                        slot.build = None;
                    }
                }
                Some(BuiltInstall::BreakerPopBox {
                    cooldown,
                    flash_timer,
                    build_timer,
                }) => {
                    *cooldown = (*cooldown - dt).max(0.0);
                    *flash_timer = (*flash_timer - dt).max(0.0);
                    *build_timer = (*build_timer - dt).max(0.0);
                }
                _ => {}
            }
        }

        for explosion in &mut self.explosions {
            explosion.timer = (explosion.timer - dt).max(0.0);
        }
        self.explosions.retain(|explosion| explosion.timer > 0.0);
    }

    fn update_spawns(&mut self, dt: f32) {
        if self.spawn_cursor >= self.pending_spawns.len() {
            return;
        }

        self.spawn_timer -= dt;
        if self.spawn_timer > 0.0 {
            return;
        }

        let lane_id = self.pending_spawns[self.spawn_cursor];
        let start = self.lanes[lane_id].waypoints[0];
        self.enemies.push(Enemy::new(lane_id, start));
        self.spawn_cursor += 1;

        if self.spawn_cursor < self.pending_spawns.len() {
            self.spawn_timer = WAVE_PLANS[self.waves_started - 1].spawn_interval;
        }
    }

    fn trigger_traps(&mut self) {
        let mut hits = Vec::new();

        for slot_index in 0..self.slots.len() {
            let lane_id = self.slots[slot_index].lane_id;
            let center = self.slots[slot_index].center();
            let Some(BuiltInstall::BreakerPopBox { cooldown, .. }) =
                self.slots[slot_index].build.as_mut()
            else {
                continue;
            };

            if *cooldown > 0.0 {
                continue;
            }

            if let Some(enemy_index) = self.enemies.iter().position(|enemy| {
                enemy.lane_id == lane_id
                    && matches!(enemy.state, EnemyState::Walking | EnemyState::BreakingDoor)
                    && enemy.position.distance(center) <= POP_BOX_RANGE
            }) {
                *cooldown = POP_BOX_COOLDOWN;
                hits.push((slot_index, enemy_index));
            }
        }

        for (slot_index, enemy_index) in hits {
            if let Some(enemy) = self.enemies.get_mut(enemy_index) {
                enemy.hp -= POP_BOX_DAMAGE;
                enemy.slow_timer = enemy.slow_timer.max(POP_BOX_SLOW_DURATION);
                self.explosions.push(Explosion {
                    position: enemy_payload_position(enemy),
                    timer: EXPLOSION_DURATION,
                });
            }

            if let Some(BuiltInstall::BreakerPopBox { flash_timer, .. }) =
                self.slots[slot_index].build.as_mut()
            {
                *flash_timer = 0.22;
            }
        }
    }

    fn update_enemies(&mut self, dt: f32) {
        let mut survivors = Vec::with_capacity(self.enemies.len());
        let enemies = std::mem::take(&mut self.enemies);

        for mut enemy in enemies {
            enemy.slow_timer = (enemy.slow_timer - dt).max(0.0);
            enemy.attack_timer = (enemy.attack_timer - dt).max(0.0);
            enemy.anim_timer += dt
                * if enemy.state == EnemyState::Installing {
                    2.0
                } else {
                    8.0
                };

            if enemy.hp <= 0.0 {
                self.scrap += CONTRACTOR_SCRAP_REWARD;
                self.contractors_stopped += 1;
                continue;
            }

            match enemy.state {
                EnemyState::Walking => {
                    if self.enemy_attacks_fence(&mut enemy) {
                        if enemy.hp > 0.0 {
                            survivors.push(enemy);
                        }
                        continue;
                    }

                    let lane = &self.lanes[enemy.lane_id];
                    if enemy.next_waypoint >= lane.waypoints.len() {
                        enemy.state = EnemyState::Installing;
                    } else {
                        let target = lane.waypoints[enemy.next_waypoint];
                        let delta = target - enemy.position;
                        let distance = delta.length();
                        let speed = if enemy.slow_timer > 0.0 {
                            CONTRACTOR_SLOWED_SPEED
                        } else {
                            CONTRACTOR_SPEED
                        };
                        let step = speed * dt;

                        if distance > 0.0 {
                            enemy.facing = delta / distance;
                        }

                        if distance <= step {
                            enemy.position = target;

                            if enemy.next_waypoint == 1 && !self.doors[enemy.lane_id].broken {
                                enemy.next_waypoint = 2;
                                enemy.state = EnemyState::BreakingDoor;
                            } else {
                                enemy.next_waypoint += 1;
                                if enemy.next_waypoint >= lane.waypoints.len() {
                                    enemy.state = EnemyState::Installing;
                                }
                            }
                        } else if distance > 0.0 {
                            enemy.position += enemy.facing * step;
                        }
                    }
                }
                EnemyState::BreakingDoor => {
                    let door = &mut self.doors[enemy.lane_id];
                    enemy.position = door.rect.center();

                    if door.broken {
                        enemy.state = EnemyState::Walking;
                    } else if enemy.attack_timer <= 0.0 {
                        door.hp -= DOOR_DAMAGE_PER_HIT;
                        door.flash_timer = HIT_FLASH_DURATION;
                        enemy.attack_timer = DOOR_DAMAGE_INTERVAL;
                        self.message = format!(
                            "{} crew is hammering the entry door.",
                            self.lanes[enemy.lane_id].name
                        );

                        if door.hp <= 0.0 {
                            door.hp = 0.0;
                            door.broken = true;
                            self.message =
                                format!("{} door breached.", self.lanes[enemy.lane_id].name);
                            self.explosions.push(Explosion {
                                position: door.rect.center(),
                                timer: EXPLOSION_DURATION * 0.8,
                            });
                        }
                    }
                }
                EnemyState::Installing => {
                    enemy.install_timer += dt;
                    if enemy.install_timer >= CONTRACTOR_INSTALL_TIME {
                        self.conversion += CONTRACTOR_INSTALL_CONVERSION;
                        self.message = format!(
                            "{} crew rolled more servers onto the floor.",
                            self.lanes[enemy.lane_id].name
                        );
                        continue;
                    }
                }
            }

            if enemy.hp > 0.0 {
                survivors.push(enemy);
            }
        }

        self.enemies = survivors;
    }

    fn enemy_attacks_fence(&mut self, enemy: &mut Enemy) -> bool {
        let Some(slot_index) = self.find_fence_slot(enemy.lane_id) else {
            return false;
        };

        let fence_center = self.slots[slot_index].center();
        let Some(BuiltInstall::EventFence { hp, hit_flash, .. }) =
            self.slots[slot_index].build.as_mut()
        else {
            return false;
        };

        if enemy.position.distance(fence_center) > EVENT_FENCE_CONTACT_RANGE {
            return false;
        }

        enemy.facing = facing_to(enemy.position, fence_center);

        if enemy.attack_timer <= 0.0 {
            *hp -= EVENT_FENCE_DAMAGE_PER_HIT;
            *hit_flash = HIT_FLASH_DURATION;
            enemy.attack_timer = EVENT_FENCE_DAMAGE_INTERVAL;
            if *hp <= 0.0 {
                self.message = format!("{} fence line collapsed.", self.lanes[enemy.lane_id].name);
                self.explosions.push(Explosion {
                    position: fence_center,
                    timer: EXPLOSION_DURATION * 0.7,
                });
                self.slots[slot_index].build = None;
            }
        }

        true
    }

    fn find_fence_slot(&self, lane_id: usize) -> Option<usize> {
        self.slots.iter().position(|slot| {
            slot.lane_id == lane_id && matches!(slot.build, Some(BuiltInstall::EventFence { .. }))
        })
    }

    fn slot_at_point(&self, point: Vec2) -> Option<usize> {
        self.slots
            .iter()
            .position(|slot| slot.rect.contains_point(point))
    }

    fn draw_floor(
        &self,
        canvas: &mut Canvas,
        view: &RenderView,
        show_route_overlay: bool,
        show_floor_grid: bool,
    ) {
        let outer = view.rect(Self::world_bounds());
        canvas.rect(
            outer.x,
            outer.y,
            outer.width,
            outer.height,
            Color::from_rgba8(77, 27, 36, 255),
        );

        let inner = view.rect(Rect::new(
            WORLD_LEFT + 8.0,
            WORLD_BOTTOM + 8.0,
            WORLD_WIDTH - 16.0,
            WORLD_HEIGHT - 16.0,
        ));
        canvas.rect(
            inner.x,
            inner.y,
            inner.width,
            inner.height,
            Color::from_rgba8(126, 58, 58, 255),
        );

        if show_floor_grid {
            for line in 0..=20 {
                let x = WORLD_LEFT + 20.0 + line as f32 * 40.0;
                let a = view.point(Vec2::new(x, WORLD_BOTTOM + 12.0));
                canvas.rect(
                    a.x,
                    inner.y,
                    view.scalar(1.0).max(1.0),
                    inner.height,
                    Color::from_rgba8(187, 109, 86, 120),
                );
            }
            for line in 0..=12 {
                let y = WORLD_BOTTOM + 20.0 + line as f32 * 40.0;
                let a = view.point(Vec2::new(WORLD_LEFT + 8.0, y));
                canvas.rect(
                    inner.x,
                    a.y,
                    inner.width,
                    view.scalar(1.0).max(1.0),
                    Color::from_rgba8(187, 109, 86, 120),
                );
            }
        }

        for bay in &self.server_bays {
            let rack = view.rect(*bay);
            canvas.rect(
                rack.x,
                rack.y,
                rack.width,
                rack.height,
                Color::from_rgba8(43, 94, 137, 155),
            );
            canvas.rect(
                rack.x + view.scalar(4.0),
                rack.y + view.scalar(4.0),
                rack.width - view.scalar(8.0),
                rack.height - view.scalar(8.0),
                Color::from_rgba8(110, 188, 220, 115),
            );
        }

        if show_route_overlay {
            for lane in &self.lanes {
                for segment in lane.waypoints.windows(2) {
                    draw_route(
                        canvas,
                        view,
                        segment[0],
                        segment[1],
                        10.0,
                        Color::from_rgba8(232, 168, 82, 125),
                    );
                }
            }
        }

        let title = view.point(Vec2::new(420.0, 234.0));
        canvas.text_aligned(
            title.x,
            title.y,
            "OPEN SERVER FLOOR",
            view.font_size(18.0),
            Color::from_rgba8(255, 236, 205, 255),
            TextAlign::Center,
        );

        if show_route_overlay {
            for lane in &self.lanes {
                let start = lane.waypoints[1];
                let label = view.point(Vec2::new(start.x, start.y + 24.0));
                canvas.text_aligned(
                    label.x,
                    label.y,
                    lane.entry_label,
                    view.font_size(10.0),
                    Color::from_rgba8(255, 223, 190, 255),
                    TextAlign::Center,
                );
            }
        }
    }

    fn draw_objective(&self, canvas: &mut Canvas, view: &RenderView) {
        let objective = view.rect(self.objective_rect);
        let objective_color = if self.conversion >= 75.0 {
            Color::from_rgba8(216, 81, 67, 255)
        } else {
            Color::from_rgba8(48, 102, 196, 255)
        };

        canvas.rect(
            objective.x,
            objective.y,
            objective.width,
            objective.height,
            objective_color,
        );
        canvas.rect(
            objective.x + view.scalar(10.0),
            objective.y + view.scalar(10.0),
            objective.width - view.scalar(20.0),
            objective.height - view.scalar(20.0),
            Color::from_rgba8(26, 41, 86, 255),
        );

        for row in 0..3 {
            for col in 0..3 {
                let cell = Rect::new(
                    self.objective_rect.x + 18.0 + col as f32 * 28.0,
                    self.objective_rect.y + 18.0 + row as f32 * 28.0,
                    18.0,
                    18.0,
                );
                let cell = view.rect(cell);
                canvas.rect(
                    cell.x,
                    cell.y,
                    cell.width,
                    cell.height,
                    Color::from_rgba8(105, 203, 255, 255),
                );
            }
        }

        canvas.text_aligned(
            objective.center().x,
            objective.top() + view.scalar(16.0),
            "Install Bays",
            view.font_size(11.0),
            Color::from_rgba8(255, 239, 213, 255),
            TextAlign::Center,
        );

        let fill = (self.conversion / MAX_CONVERSION).clamp(0.0, 1.0) * objective.width;
        canvas.rect(
            objective.x,
            objective.y - view.scalar(12.0),
            objective.width,
            view.scalar(7.0),
            Color::from_rgba8(70, 22, 20, 255),
        );
        canvas.rect(
            objective.x,
            objective.y - view.scalar(12.0),
            fill,
            view.scalar(7.0),
            Color::from_rgba8(242, 188, 95, 255),
        );
    }

    fn draw_doors(&self, canvas: &mut Canvas, view: &RenderView) {
        for (lane_id, door) in self.doors.iter().enumerate() {
            let rect = view.rect(door.rect);
            let flash = door.flash_timer / HIT_FLASH_DURATION;
            let door_color = if door.broken {
                Color::from_rgba8(118, 53, 43, 255)
            } else if flash > 0.0 {
                Color::from_rgba8(243, 176, 94, 255)
            } else {
                Color::from_rgba8(169, 73, 56, 255)
            };

            if door.broken {
                if rect.width > rect.height {
                    canvas.rect(rect.x, rect.y, rect.width * 0.4, rect.height, door_color);
                    canvas.rect(
                        rect.right() - rect.width * 0.4,
                        rect.y,
                        rect.width * 0.4,
                        rect.height,
                        door_color,
                    );
                } else {
                    canvas.rect(rect.x, rect.y, rect.width, rect.height * 0.4, door_color);
                    canvas.rect(
                        rect.x,
                        rect.top() - rect.height * 0.4,
                        rect.width,
                        rect.height * 0.4,
                        door_color,
                    );
                }
            } else {
                canvas.rect(rect.x, rect.y, rect.width, rect.height, door_color);
                if rect.width > rect.height {
                    canvas.rect(
                        rect.center().x - view.scalar(2.0),
                        rect.y,
                        view.scalar(4.0),
                        rect.height,
                        Color::from_rgba8(92, 28, 24, 255),
                    );
                } else {
                    canvas.rect(
                        rect.x,
                        rect.center().y - view.scalar(2.0),
                        rect.width,
                        view.scalar(4.0),
                        Color::from_rgba8(92, 28, 24, 255),
                    );
                }
            }

            let hp_fill = (door.hp / DOOR_MAX_HP).clamp(0.0, 1.0) * rect.width;
            canvas.rect(
                rect.x,
                rect.top() + view.scalar(5.0),
                rect.width,
                view.scalar(4.0),
                Color::from_rgba8(72, 19, 19, 255),
            );
            canvas.rect(
                rect.x,
                rect.top() + view.scalar(5.0),
                hp_fill,
                view.scalar(4.0),
                Color::from_rgba8(233, 197, 117, 255),
            );

            let label = view.point(self.lanes[lane_id].waypoints[1] + Vec2::new(0.0, 18.0));
            canvas.text_aligned(
                label.x,
                label.y,
                if door.broken { "BREACHED" } else { "DOOR" },
                view.font_size(8.0),
                Color::from_rgba8(255, 227, 201, 255),
                TextAlign::Center,
            );
        }
    }

    fn draw_slots(&self, canvas: &mut Canvas, view: &RenderView, hovered_slot: Option<usize>) {
        for (slot_index, slot) in self.slots.iter().enumerate() {
            let base_color = match slot.kind {
                BuildSlotKind::Fence => Color::from_rgba8(180, 74, 59, 190),
                BuildSlotKind::Trap => Color::from_rgba8(242, 162, 57, 195),
            };
            let slot_rect = view.rect(slot.rect);

            if hovered_slot == Some(slot_index) {
                let outline = view.rect(Rect::new(
                    slot.rect.x - 5.0,
                    slot.rect.y - 5.0,
                    slot.rect.width + 10.0,
                    slot.rect.height + 10.0,
                ));
                canvas.rect(
                    outline.x,
                    outline.y,
                    outline.width,
                    outline.height,
                    Color::from_rgba8(245, 223, 161, 255),
                );
            }

            canvas.rect(
                slot_rect.x,
                slot_rect.y,
                slot_rect.width,
                slot_rect.height,
                base_color,
            );

            match slot.build.as_ref() {
                None => {
                    let center = view.point(slot.center());
                    canvas.text_aligned(
                        center.x,
                        center.y + view.scalar(4.0),
                        slot.build_short_label(),
                        view.font_size(8.0),
                        Color::from_rgba8(89, 25, 19, 255),
                        TextAlign::Center,
                    );
                }
                Some(BuiltInstall::EventFence {
                    hp,
                    build_timer,
                    hit_flash,
                }) => {
                    let pulse =
                        (*build_timer / BUILD_FLASH_DURATION).max(*hit_flash / HIT_FLASH_DURATION);
                    if pulse > 0.0 {
                        let glow = view.rect(Rect::new(
                            slot.rect.x - 8.0,
                            slot.rect.y - 8.0,
                            slot.rect.width + 16.0,
                            slot.rect.height + 16.0,
                        ));
                        canvas.rect(
                            glow.x,
                            glow.y,
                            glow.width,
                            glow.height,
                            Color::from_rgba8(255, 223, 155, (pulse * 180.0) as u8),
                        );
                    }

                    canvas.rect(
                        slot_rect.x,
                        slot_rect.y,
                        slot_rect.width,
                        slot_rect.height,
                        Color::from_rgba8(171, 72, 58, 255),
                    );
                    canvas.rect(
                        slot_rect.x + view.scalar(5.0),
                        slot_rect.y + view.scalar(4.0),
                        slot_rect.width - view.scalar(10.0),
                        view.scalar(6.0),
                        Color::from_rgba8(240, 204, 142, 255),
                    );
                    canvas.rect(
                        slot_rect.x + view.scalar(5.0),
                        slot_rect.top() - view.scalar(10.0),
                        slot_rect.width - view.scalar(10.0),
                        view.scalar(6.0),
                        Color::from_rgba8(240, 204, 142, 255),
                    );

                    let fill = (*hp / EVENT_FENCE_MAX_HP).clamp(0.0, 1.0) * slot_rect.width;
                    canvas.rect(
                        slot_rect.x,
                        slot_rect.top() + view.scalar(6.0),
                        slot_rect.width,
                        view.scalar(4.0),
                        Color::from_rgba8(77, 22, 19, 255),
                    );
                    canvas.rect(
                        slot_rect.x,
                        slot_rect.top() + view.scalar(6.0),
                        fill,
                        view.scalar(4.0),
                        Color::from_rgba8(112, 214, 132, 255),
                    );
                }
                Some(BuiltInstall::BreakerPopBox {
                    flash_timer,
                    build_timer,
                    ..
                }) => {
                    let pulse = (*build_timer / BUILD_FLASH_DURATION).max(*flash_timer / 0.22);
                    if pulse > 0.0 {
                        let glow = view.rect(Rect::new(
                            slot.rect.x - 7.0,
                            slot.rect.y - 7.0,
                            slot.rect.width + 14.0,
                            slot.rect.height + 14.0,
                        ));
                        canvas.rect(
                            glow.x,
                            glow.y,
                            glow.width,
                            glow.height,
                            Color::from_rgba8(255, 224, 132, (pulse * 180.0) as u8),
                        );
                    }

                    canvas.rect(
                        slot_rect.x,
                        slot_rect.y,
                        slot_rect.width,
                        slot_rect.height,
                        if *flash_timer > 0.0 {
                            Color::from_rgba8(255, 208, 103, 255)
                        } else {
                            Color::from_rgba8(249, 163, 46, 255)
                        },
                    );
                    canvas.rect(
                        slot_rect.x + view.scalar(6.0),
                        slot_rect.y + view.scalar(6.0),
                        slot_rect.width - view.scalar(12.0),
                        slot_rect.height - view.scalar(12.0),
                        Color::from_rgba8(110, 33, 24, 255),
                    );
                    let center = view.point(slot.center());
                    canvas.text_aligned(
                        center.x,
                        center.y + view.scalar(4.0),
                        "!",
                        view.font_size(10.0),
                        Color::from_rgba8(255, 230, 145, 255),
                        TextAlign::Center,
                    );
                }
            }
        }
    }

    fn draw_enemies(&self, canvas: &mut Canvas, view: &RenderView) {
        for enemy in &self.enemies {
            self.draw_enemy(canvas, view, enemy);
        }
    }

    fn draw_enemy(&self, canvas: &mut Canvas, view: &RenderView, enemy: &Enemy) {
        let body = view.point(enemy.position);
        let facing = normalized_or(enemy.facing, Vec2::new(1.0, 0.0));
        let side = Vec2::new(-facing.y, facing.x);
        let cart = view.point(enemy_payload_position(enemy));
        let stride = enemy.anim_timer.sin() * view.scalar(2.0);
        let pixel = view.scalar(2.0).max(2.0);
        let shadow = view.scalar(11.0);

        canvas.rect(
            cart.x - shadow * 0.6,
            cart.y - view.scalar(4.0),
            shadow * 1.2,
            view.scalar(4.0),
            Color::from_rgba8(0, 0, 0, 90),
        );
        canvas.rect(
            body.x - shadow * 0.45,
            body.y - view.scalar(5.0),
            shadow * 0.9,
            view.scalar(4.0),
            Color::from_rgba8(0, 0, 0, 90),
        );

        let cart_color = if enemy.state == EnemyState::Installing {
            Color::from_rgba8(204, 93, 67, 255)
        } else {
            Color::from_rgba8(63, 123, 206, 255)
        };
        canvas.rect(
            cart.x - view.scalar(11.0),
            cart.y - view.scalar(7.0),
            view.scalar(18.0),
            view.scalar(14.0),
            cart_color,
        );
        canvas.rect(
            cart.x - view.scalar(13.0),
            cart.y - view.scalar(9.0),
            view.scalar(22.0),
            view.scalar(3.0),
            Color::from_rgba8(28, 66, 119, 255),
        );
        canvas.rect(
            cart.x - view.scalar(14.0),
            cart.y - view.scalar(12.0),
            view.scalar(4.0),
            view.scalar(20.0),
            Color::from_rgba8(100, 37, 29, 255),
        );
        canvas.rect(
            cart.x + view.scalar(10.0),
            cart.y - view.scalar(12.0),
            view.scalar(4.0),
            view.scalar(20.0),
            Color::from_rgba8(100, 37, 29, 255),
        );
        canvas.rect(
            cart.x - view.scalar(13.0),
            cart.y + view.scalar(8.0),
            view.scalar(4.0),
            view.scalar(4.0),
            Color::from_rgba8(71, 22, 18, 255),
        );
        canvas.rect(
            cart.x + view.scalar(9.0),
            cart.y + view.scalar(8.0),
            view.scalar(4.0),
            view.scalar(4.0),
            Color::from_rgba8(71, 22, 18, 255),
        );

        let leg_offset = if enemy.state == EnemyState::Walking {
            stride
        } else {
            0.0
        };
        canvas.rect(
            body.x - pixel * 2.0,
            body.y - pixel * 5.0,
            pixel * 2.0,
            pixel * 4.0 + leg_offset.abs(),
            Color::from_rgba8(34, 44, 86, 255),
        );
        canvas.rect(
            body.x,
            body.y - pixel * 5.0,
            pixel * 2.0,
            pixel * 4.0 + leg_offset.abs(),
            Color::from_rgba8(34, 44, 86, 255),
        );
        canvas.rect(
            body.x - pixel * 3.0,
            body.y - pixel * 2.0,
            pixel * 6.0,
            pixel * 5.0,
            if enemy.state == EnemyState::BreakingDoor {
                Color::from_rgba8(203, 135, 72, 255)
            } else {
                Color::from_rgba8(231, 148, 81, 255)
            },
        );
        canvas.rect(
            body.x - pixel * 2.0,
            body.y + pixel * 3.0,
            pixel * 4.0,
            pixel * 2.0,
            Color::from_rgba8(243, 210, 119, 255),
        );
        canvas.rect(
            body.x - pixel * 2.0,
            body.y + pixel * 5.0,
            pixel * 4.0,
            pixel * 3.0,
            Color::from_rgba8(213, 184, 149, 255),
        );

        let arm_target = body + facing * view.scalar(6.0) + side * view.scalar(2.0);
        canvas.rect(
            body.x - pixel * 5.0,
            body.y,
            pixel * 2.0,
            pixel * 4.0,
            Color::from_rgba8(213, 184, 149, 255),
        );
        canvas.rect(
            arm_target.x - pixel,
            arm_target.y - pixel,
            pixel * 2.0,
            pixel * 4.0,
            Color::from_rgba8(213, 184, 149, 255),
        );

        let hp_fill = (enemy.hp / CONTRACTOR_HP).clamp(0.0, 1.0) * view.scalar(20.0);
        canvas.rect(
            body.x - view.scalar(10.0),
            body.y + view.scalar(13.0),
            view.scalar(20.0),
            view.scalar(3.0),
            Color::from_rgba8(32, 24, 18, 255),
        );
        canvas.rect(
            body.x - view.scalar(10.0),
            body.y + view.scalar(13.0),
            hp_fill,
            view.scalar(3.0),
            if enemy.slow_timer > 0.0 {
                Color::from_rgba8(129, 208, 255, 255)
            } else {
                Color::from_rgba8(134, 232, 143, 255)
            },
        );
    }

    fn draw_explosions(&self, canvas: &mut Canvas, view: &RenderView) {
        for explosion in &self.explosions {
            let center = view.point(explosion.position);
            let progress = 1.0 - explosion.timer / EXPLOSION_DURATION;
            let radius = view.scalar(8.0 + progress * 28.0);
            let alpha = ((1.0 - progress) * 210.0) as u8;

            canvas.rect(
                center.x - radius,
                center.y - view.scalar(3.0),
                radius * 2.0,
                view.scalar(6.0),
                Color::from_rgba8(255, 209, 111, alpha),
            );
            canvas.rect(
                center.x - view.scalar(3.0),
                center.y - radius,
                view.scalar(6.0),
                radius * 2.0,
                Color::from_rgba8(255, 171, 87, alpha),
            );
            canvas.rect(
                center.x - radius * 0.55,
                center.y - radius * 0.55,
                radius * 1.1,
                radius * 1.1,
                Color::from_rgba8(255, 240, 188, alpha.saturating_sub(40)),
            );
        }
    }

    fn draw_cursor(&self, canvas: &mut Canvas, view: &RenderView, mouse: Vec2) {
        let pos = view.point(mouse);
        let arm = view.scalar(10.0);
        let thickness = view.scalar(2.0).max(2.0);
        canvas.rect(
            pos.x - thickness * 0.5,
            pos.y - arm,
            thickness,
            arm * 2.0,
            Color::from_rgba8(255, 241, 188, 210),
        );
        canvas.rect(
            pos.x - arm,
            pos.y - thickness * 0.5,
            arm * 2.0,
            thickness,
            Color::from_rgba8(255, 241, 188, 210),
        );
    }
}

fn draw_route(
    canvas: &mut Canvas,
    view: &RenderView,
    start: Vec2,
    end: Vec2,
    thickness: f32,
    color: Color,
) {
    let start = view.point(start);
    let end = view.point(end);
    let thickness = view.scalar(thickness);

    if (start.x - end.x).abs() <= f32::EPSILON {
        let x = start.x - thickness * 0.5;
        let y = start.y.min(end.y);
        let h = (end.y - start.y).abs().max(1.0);
        canvas.rect(x, y, thickness, h, color);
    } else {
        let x = start.x.min(end.x);
        let y = start.y - thickness * 0.5;
        let w = (end.x - start.x).abs().max(1.0);
        canvas.rect(x, y, w, thickness, color);
    }
}

fn normalized_or(value: Vec2, fallback: Vec2) -> Vec2 {
    let length = value.length();
    if length > 0.0 {
        value / length
    } else {
        fallback
    }
}

fn facing_to(from: Vec2, to: Vec2) -> Vec2 {
    normalized_or(to - from, Vec2::new(1.0, 0.0))
}

fn enemy_payload_position(enemy: &Enemy) -> Vec2 {
    let facing = normalized_or(enemy.facing, Vec2::new(1.0, 0.0));
    let side = Vec2::new(-facing.y, facing.x);
    enemy.position - facing * 16.0 + side * 7.0
}

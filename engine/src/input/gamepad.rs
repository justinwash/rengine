use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use std::collections::HashMap;
use winit::keyboard::KeyCode;

use super::keyboard::InputState;

pub const MAX_PLAYERS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamepadAssignMode {
    OnConnect,
    OnButtonPress,
}

impl Default for GamepadAssignMode {
    fn default() -> Self {
        Self::OnButtonPress
    }
}

/// A rengine-owned handle for \"this pad occupies this slot\", so connectedness
/// is testable without a real gilrs `GamepadId` (which only gilrs can build).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GamepadToken(pub(crate) u32);

#[derive(Debug, Clone)]
pub struct GamepadState {
    pub(crate) id: Option<GamepadToken>,

    buttons_down: Vec<Button>,

    buttons_pressed: Vec<Button>,

    buttons_released: Vec<Button>,

    pub left_stick_x: f32,

    pub left_stick_y: f32,

    pub right_stick_x: f32,

    pub right_stick_y: f32,

    /// The [`GamepadSystem`] update tick this pad last produced input on.
    /// A pad silent for long enough counts as gone, so a replacement
    /// controller can take its slot (see `assign_slot`).
    last_event_epoch: u64,
}

impl GamepadState {
    pub const DEFAULT: Self = Self {
        id: None,
        buttons_down: Vec::new(),
        buttons_pressed: Vec::new(),
        buttons_released: Vec::new(),
        left_stick_x: 0.0,
        left_stick_y: 0.0,
        right_stick_x: 0.0,
        right_stick_y: 0.0,
        last_event_epoch: 0,
    };

    pub fn new() -> Self {
        Self::DEFAULT
    }

    pub fn is_button_down(&self, button: Button) -> bool {
        self.buttons_down.contains(&button)
    }

    pub fn is_button_pressed(&self, button: Button) -> bool {
        self.buttons_pressed.contains(&button)
    }

    pub fn is_button_released(&self, button: Button) -> bool {
        self.buttons_released.contains(&button)
    }

    pub fn is_connected(&self) -> bool {
        self.id.is_some()
    }

    /// Whether any button went down this frame — the cheap test the input
    /// device-vote uses to tell "the pad is being used" from a resting pad.
    pub fn any_button_pressed(&self) -> bool {
        !self.buttons_pressed.is_empty()
    }
}

pub struct GamepadSystem {
    gilrs: Gilrs,

    pub(crate) slots: Vec<GamepadState>,

    id_to_slot: HashMap<GamepadId, usize>,

    unassigned: Vec<GamepadId>,

    assign_mode: GamepadAssignMode,

    /// Monotonic update ticks, used to spot pads that stopped producing input.
    epoch: u64,
}

/// A pad this silent (ticks at 60fps ≈ 10s) counts as abandoned for the
/// purpose of slot assignment. A controller whose battery died while the link
/// stayed up would otherwise hold the primary player slot hostage while a
/// replacement pad waits on a later slot.
/// ponytail: activity-based, so a genuine two-pad session with one player
/// idle for >10s can see the idle pad re-homed to a later slot — fine for the
/// single-player game; revisit with per-slot pairing if real 2P ever ships.
const STALE_SLOT_EPOCHS: u64 = 600;

impl GamepadSystem {
    pub fn new(mode: GamepadAssignMode) -> Self {
        let gilrs = Gilrs::new().expect("Failed to initialise gilrs");
        let mut sys = Self {
            gilrs,
            slots: (0..MAX_PLAYERS).map(|_| GamepadState::new()).collect(),
            id_to_slot: HashMap::new(),
            unassigned: Vec::new(),
            assign_mode: mode,
            epoch: 0,
        };

        let connected: Vec<GamepadId> = sys
            .gilrs
            .gamepads()
            .filter(|(_, gp)| gp.is_connected())
            .map(|(id, _)| id)
            .collect();
        for id in connected {
            sys.track_gamepad(id);
        }
        sys
    }

    pub fn assign_mode(&self) -> GamepadAssignMode {
        self.assign_mode
    }

    pub fn set_assign_mode(&mut self, mode: GamepadAssignMode) {
        self.assign_mode = mode;
        if mode == GamepadAssignMode::OnConnect {
            let pending: Vec<GamepadId> = self.unassigned.drain(..).collect();
            for id in pending {
                self.assign_slot(id);
            }
        }
    }

    pub fn unassigned_count(&self) -> usize {
        self.unassigned.len()
    }

    pub fn player(&self, index: usize) -> &GamepadState {
        &self.slots[index]
    }

    pub fn player_or_default(&self, index: usize) -> &GamepadState {
        static DEFAULT: GamepadState = GamepadState::DEFAULT;
        self.slots.get(index).unwrap_or(&DEFAULT)
    }

    pub fn connected_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_connected()).count()
    }

    /// Merge a connected player-0 pad into the keyboard state the game reads.
    ///
    /// The face/menu buttons map to the universal keys the whole input scheme
    /// already keys off (A→Enter, B→Esc, Start→Space, View→Esc), so **every**
    /// existing `is_key_pressed(Enter)` / `Escape` / `Space` call site becomes
    /// controller-aware with no game-side edit — the same translation a
    /// platform layer (Steam Input, XInput) performs. Direction is *not*
    /// merged: the D-pad means "move the highlight" in menus but "walk the
    /// focus ring" in the race, so the game reads the pad directly where the
    /// two split.
    pub(crate) fn translate_to_keys(&self, input: &mut InputState) {
        let Some(gp) = self.slots.get(0) else {
            return;
        };
        if !gp.is_connected() {
            return;
        }
        if gp.is_button_pressed(Button::South) {
            input.inject_key_press(KeyCode::Enter);
        }
        if gp.is_button_pressed(Button::East) {
            input.inject_key_press(KeyCode::Escape);
        }
        if gp.is_button_pressed(Button::Start) {
            input.inject_key_press(KeyCode::Space);
        }
        // View is the scheme's "deeper out": on screens without a back target
        // it opens the pause menu, on screens with one it goes back — which is
        // exactly what Esc already resolves to per screen.
        if gp.is_button_pressed(Button::Select) {
            input.inject_key_press(KeyCode::Escape);
        }
    }

    pub(crate) fn update(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        for slot in &mut self.slots {
            slot.buttons_pressed.clear();
            slot.buttons_released.clear();
        }

        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    // A controller plugged in (or re-paired) mid-session joins
                    // the pool — without this a replacement pad would never be
                    // tracked until restart.
                    self.track_gamepad(event.id);
                }
                EventType::Disconnected => {
                    self.unassigned.retain(|&id| id != event.id);
                    if let Some(&slot_idx) = self.id_to_slot.get(&event.id) {
                        let slot = &mut self.slots[slot_idx];
                        slot.id = None;
                        slot.buttons_down.clear();
                        slot.buttons_pressed.clear();
                        slot.buttons_released.clear();
                        self.id_to_slot.remove(&event.id);
                    }
                }
                EventType::ButtonPressed(button, _) => {
                    // Any press from a pad we do not yet own claims it: the
                    // "wait for a press to assign" mode, a pad freed by a slot
                    // reclaim (it went silent long enough, then the player came
                    // back), or simply a fresh controller. Dropping a press
                    // because the id fell out of `unassigned` is how a swap
                    // could look dead.
                    if !self.id_to_slot.contains_key(&event.id) {
                        self.unassigned.retain(|&id| id != event.id);
                        self.assign_slot(event.id);
                    }
                    if let Some(&slot_idx) = self.id_to_slot.get(&event.id) {
                        let slot = &mut self.slots[slot_idx];
                        slot.last_event_epoch = self.epoch;
                        if !slot.buttons_down.contains(&button) {
                            slot.buttons_down.push(button);
                            slot.buttons_pressed.push(button);
                        }
                    }
                }
                EventType::ButtonReleased(button, _) => {
                    if let Some(&slot_idx) = self.id_to_slot.get(&event.id) {
                        let slot = &mut self.slots[slot_idx];
                        slot.last_event_epoch = self.epoch;
                        slot.buttons_down.retain(|&b| b != button);
                        slot.buttons_released.push(button);
                    }
                }
                // Analog pads talk in axis/button-changed events only; stamping
                // here is what keeps them alive for the silent-slot reclaim.
                // A pad used *without a button press* (just the stick) would
                // otherwise never leave `unassigned` in OnButtonPress mode —
                // meaningful deflection counts as the handshake too.
                EventType::AxisChanged(_, value, _) | EventType::ButtonChanged(_, value, _) => {
                    if !self.id_to_slot.contains_key(&event.id) && value.abs() > 0.5 {
                        self.unassigned.retain(|&id| id != event.id);
                        self.assign_slot(event.id);
                    }
                    if let Some(&slot_idx) = self.id_to_slot.get(&event.id) {
                        self.slots[slot_idx].last_event_epoch = self.epoch;
                    }
                }
                _ => {}
            }
        }

        // Free slots whose pad has gone silent, then pack everything up so the
        // first-player slot is always the live pad the game reads. Running
        // every update (not just when a new pad assigns) means a dead pad's
        // slot opens up even when no new assignment is pending, and the
        // promotion below hands primary to whichever pad is used.
        self.reclaim_stale_slots();
        self.renormalize_slots();

        // Stick state is refreshed from the live gilrs value for every
        // assigned slot. The `id_to_slot` map is the source of the gilrs id
        // (the slot's own token only says "occupied").
        let assigned: Vec<(usize, GamepadId)> = self
            .id_to_slot
            .iter()
            .map(|(&id, &slot_idx)| (slot_idx, id))
            .collect();
        for (slot_idx, gid) in assigned {
            let Some(gp) = self.gilrs.connected_gamepad(gid) else {
                continue;
            };
            let slot = &mut self.slots[slot_idx];
            slot.left_stick_x = gp.value(Axis::LeftStickX);
            slot.left_stick_y = gp.value(Axis::LeftStickY);
            slot.right_stick_x = gp.value(Axis::RightStickX);
            slot.right_stick_y = gp.value(Axis::RightStickY);

            if gp.is_pressed(Button::DPadLeft) {
                slot.left_stick_x = -1.0;
            } else if gp.is_pressed(Button::DPadRight) {
                slot.left_stick_x = 1.0;
            }
            if gp.is_pressed(Button::DPadUp) {
                slot.left_stick_y = 1.0;
            } else if gp.is_pressed(Button::DPadDown) {
                slot.left_stick_y = -1.0;
            }

            if slot.left_stick_x.abs() < 0.15 {
                slot.left_stick_x = 0.0;
            }
            if slot.left_stick_y.abs() < 0.15 {
                slot.left_stick_y = 0.0;
            }
            if slot.right_stick_x.abs() < 0.15 {
                slot.right_stick_x = 0.0;
            }
            if slot.right_stick_y.abs() < 0.15 {
                slot.right_stick_y = 0.0;
            }
        }
    }

    fn track_gamepad(&mut self, id: GamepadId) {
        if self.id_to_slot.contains_key(&id) || self.unassigned.contains(&id) {
            return;
        }
        match self.assign_mode {
            GamepadAssignMode::OnConnect => self.assign_slot(id),
            GamepadAssignMode::OnButtonPress => {
                log::info!(
                    "Gamepad {:?} detected, waiting for button press to assign",
                    id
                );
                self.unassigned.push(id);
            }
        }
    }

    fn assign_slot(&mut self, id: GamepadId) {
        if self.id_to_slot.contains_key(&id) {
            return;
        }

        // A slot whose pad has gone silent is reclaimable: the battery may
        // have died with the link still up, and the replacement must become
        // the primary (slot 0) rather than wait on a later slot the
        // single-player game never reads.
        self.reclaim_stale_slots();

        for (idx, slot) in self.slots.iter_mut().enumerate() {
            if slot.id.is_none() {
                slot.id = Some(GamepadToken(idx as u32 + 1));
                // No activity stamp here: `last_event_epoch` means "this pad
                // produced real input", so a newly-plugged-but-untouched pad
                // never hijacks slot 0 from the pad being played.
                self.id_to_slot.insert(id, idx);
                log::info!("Gamepad {:?} assigned to player slot {}", id, idx + 1);
                return;
            }
        }
        log::warn!("No free player slot for gamepad {:?}", id);
    }

    /// Free every assigned slot whose pad has produced no input for
    /// [`STALE_SLOT_EPOCHS`] ticks. A controller that died without a clean
    /// `Disconnected` event would otherwise hold its player slot forever.
    fn reclaim_stale_slots(&mut self) {
        let stale: Vec<(GamepadId, usize)> = self
            .id_to_slot
            .iter()
            .filter(|(_, &slot_idx)| {
                self.epoch.saturating_sub(self.slots[slot_idx].last_event_epoch)
                    >= STALE_SLOT_EPOCHS
            })
            .map(|(&gid, &slot_idx)| (gid, slot_idx))
            .collect();
        for (gid, slot_idx) in stale {
            log::info!(
                "Gamepad {:?} silent for {} ticks; freeing player slot {}",
                gid,
                STALE_SLOT_EPOCHS,
                slot_idx + 1
            );
            self.id_to_slot.remove(&gid);
            if let Some(slot) = self.slots.get_mut(slot_idx) {
                slot.id = None;
                slot.buttons_down.clear();
                slot.buttons_pressed.clear();
                slot.buttons_released.clear();
                slot.left_stick_x = 0.0;
                slot.left_stick_y = 0.0;
                slot.right_stick_x = 0.0;
                slot.right_stick_y = 0.0;
            }
        }
    }

    /// Move pads on higher slots down into lower free slots, so the first
    /// player slot always holds a live pad. Called every update, so a slot
    /// freed by a `Disconnected` event is re-filled by the next pad on the
    /// very next frame — the game reads slot 0 and must never see an empty
    /// primary while a controller is physically connected.
    fn renormalize_slots(&mut self) {
        for i in 0..self.slots.len() {
            if self.slots[i].id.is_some() {
                continue;
            }
            let Some(j) = (i + 1..self.slots.len()).find(|&j| self.slots[j].id.is_some()) else {
                break;
            };
            // Re-point the gilrs mapping at the lower slot.
            if let Some((&gid, _)) = self.id_to_slot.iter().find(|(_, &s)| s == j) {
                self.id_to_slot.insert(gid, i);
            } else {
                self.id_to_slot
                    .retain(|_, slot_idx| *slot_idx != j);
            }
            let mut moved = std::mem::replace(&mut self.slots[j], GamepadState::new());
            moved.id = Some(GamepadToken(i as u32 + 1));
            self.slots[i] = moved;
            log::info!("Repacked gamepad into player slot {}", i + 1);
        }

        self.promote_most_active_to_primary();
    }

    /// Slot 0 belongs to the pad that produced input most recently. This is
    /// what makes a mid-session swap seamless: the instant the replacement
    /// controller's first press lands, it becomes the most recent pad and
    /// takes over the primary slot — even when the dead pad still occupies it
    /// (no `Disconnected` event, silence shorter than the reclaim threshold).
    ///
    /// A pad that has produced no input yet (`last_event_epoch == 0`) is never
    /// promoted, so plugging a spare controller in cannot yank the controls
    /// from the pad being played.
    fn promote_most_active_to_primary(&mut self) {
        if self.slots.len() < 2 || self.slots[0].id.is_none() {
            return;
        }
        let Some(candidate) = (1..self.slots.len())
            .filter(|&i| self.slots[i].id.is_some())
            .max_by_key(|&i| self.slots[i].last_event_epoch)
        else {
            return;
        };
        if self.slots[candidate].last_event_epoch <= self.slots[0].last_event_epoch {
            return; // primary is already the most recent (ties stay put)
        }
        // Re-point the gilrs mappings, then swap the whole states (including
        // the frame's recorded button presses) so the game sees the press on
        // the very frame it happened.
        let slot0_gid = self
            .id_to_slot
            .iter()
            .find(|(_, &s)| s == 0)
            .map(|(&g, _)| g);
        let cand_gid = self
            .id_to_slot
            .iter()
            .find(|(_, &s)| s == candidate)
            .map(|(&g, _)| g);
        if let Some(gid) = slot0_gid {
            self.id_to_slot.insert(gid, candidate);
        }
        if let Some(gid) = cand_gid {
            self.id_to_slot.insert(gid, 0);
        }
        self.slots.swap(0, candidate);
        self.slots[0].id = Some(GamepadToken(1));
        self.slots[candidate].id = Some(GamepadToken(candidate as u32 + 1));
        log::info!(
            "Promoted gamepad {} to player slot 1 (most recently active)",
            candidate + 1
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    /// A with a connected pad pushes the universal keys the game keys off
    /// (Enter/Esc/Space), so every existing keyboard read works on the pad.
    #[test]
    fn a_pad_translates_face_buttons_to_universal_keys() {
        let mut slot = GamepadState::DEFAULT;
        // Connected pads carry a token; stub it without a real gilrs event.
        slot.id = Some(GamepadToken(1));
        slot.buttons_pressed.push(Button::South);
        slot.buttons_pressed.push(Button::Start);

        let system = GamepadSystem {
            gilrs: Gilrs::new().expect("gilrs init"),
            slots: vec![slot],
            id_to_slot: std::collections::HashMap::new(),
            unassigned: Vec::new(),
            assign_mode: GamepadAssignMode::default(),
            epoch: 0,
        };
        // Only the first slot matters; avoid poll (it needs real events).
        let mut input = InputState::new();
        system.translate_to_keys(&mut input);
        assert!(input.is_key_pressed(KeyCode::Enter), "A is confirm");
        assert!(input.is_key_pressed(KeyCode::Space), "Start is pause");
        assert!(!input.is_key_pressed(KeyCode::Escape), "nothing pressed Esc");

        // An idle connected pad injects nothing.
        let idle = GamepadSystem {
            gilrs: Gilrs::new().expect("gilrs init"),
            slots: vec![GamepadState::DEFAULT],
            id_to_slot: std::collections::HashMap::new(),
            unassigned: Vec::new(),
            assign_mode: GamepadAssignMode::default(),
            epoch: 0,
        };
        let mut input2 = InputState::new();
        idle.translate_to_keys(&mut input2);
        assert!(
            !input2.is_key_pressed(KeyCode::Enter) && !input2.is_key_pressed(KeyCode::Space),
            "a disconnected pad must not fire keys"
        );
    }

    /// When a pad on a higher slot outlives the one on slot 0 (its controller
    /// died), the next update packs it into the now-free primary slot — the
    /// game reads slot 0 and would otherwise stay blind to the replacement.
    #[test]
    fn a_later_pad_repacks_into_the_freed_first_slot() {
        let mut sys = GamepadSystem {
            gilrs: Gilrs::new().expect("gilrs init"),
            slots: vec![
                GamepadState::DEFAULT, // slot 0 freed (its pad dropped)
                GamepadState {
                    id: Some(GamepadToken(2)),
                    last_event_epoch: 5,
                    ..GamepadState::DEFAULT
                },
            ],
            id_to_slot: std::collections::HashMap::new(),
            unassigned: Vec::new(),
            assign_mode: GamepadAssignMode::default(),
            epoch: 10,
        };
        sys.renormalize_slots();
        assert!(
            sys.slots[0].id.is_some(),
            "the surviving pad must move into the first player slot"
        );
        assert!(
            sys.slots[1].id.is_none(),
            "the source slot must be empty after the repack"
        );
    }

    /// The precise failure of a mid-session swap: the dead pad still occupies
    /// slot 0 (no Disconnected event, silence shorter than the reclaim
    /// threshold), the replacement is on slot 1 and just produced input. The
    /// most-recently-active promotion must put the replacement in slot 0 —
    /// taking its recorded button press with it — on the very next update.
    #[test]
    fn the_most_recently_active_pad_takes_primary_even_when_slot0_is_occupied() {
        let mut sys = GamepadSystem {
            gilrs: Gilrs::new().expect("gilrs init"),
            slots: vec![
                // The ghost: last input a moment before it died.
                GamepadState {
                    id: Some(GamepadToken(1)),
                    last_event_epoch: 10,
                    ..GamepadState::DEFAULT
                },
                // The replacement: its first press landed a frame ago.
                GamepadState {
                    id: Some(GamepadToken(2)),
                    last_event_epoch: 42,
                    buttons_pressed: vec![Button::South],
                    ..GamepadState::DEFAULT
                },
            ],
            id_to_slot: std::collections::HashMap::new(),
            unassigned: Vec::new(),
            assign_mode: GamepadAssignMode::default(),
            epoch: 60,
        };
        sys.renormalize_slots();
        assert_eq!(
            sys.slots[0].last_event_epoch, 42,
            "the pad that just produced input must take the primary slot"
        );
        assert_eq!(
            sys.slots[0].id,
            Some(GamepadToken(1)),
            "primary keeps the player-1 token"
        );
        assert!(
            sys.slots[0].buttons_pressed.contains(&Button::South),
            "the press that triggered the swap must still be live for the game"
        );
        assert_eq!(
            sys.slots[1].last_event_epoch, 10,
            "the ghost moves to the back slot"
        );
    }

    /// An untouched spare controller must not steal primary from the pad the
    /// player is actively using.
    #[test]
    fn an_untouched_pad_does_not_take_primary() {
        let mut sys = GamepadSystem {
            gilrs: Gilrs::new().expect("gilrs init"),
            slots: vec![
                GamepadState {
                    id: Some(GamepadToken(1)),
                    last_event_epoch: 50,
                    ..GamepadState::DEFAULT
                },
                // Plugged in but never pressed: no activity to promote on.
                GamepadState {
                    id: Some(GamepadToken(2)),
                    last_event_epoch: 0,
                    ..GamepadState::DEFAULT
                },
            ],
            id_to_slot: std::collections::HashMap::new(),
            unassigned: Vec::new(),
            assign_mode: GamepadAssignMode::default(),
            epoch: 60,
        };
        sys.renormalize_slots();
        assert_eq!(
            sys.slots[0].last_event_epoch, 50,
            "an idle spare must leave the active pad in primary"
        );
    }

    /// A pad that stopped producing input is reclaimed, so a replacement
    /// controller can take its slot even though no Disconnected event ever
    /// came (a dead battery with the link still up). Needs a real controller
    /// to supply a gilrs id; skipped (as a pass) without one.
    #[test]
    fn a_silent_controller_slot_is_reclaimed() {
        let gilrs = Gilrs::new().expect("gilrs init");
        let connected: Vec<GamepadId> = gilrs.gamepads().map(|(id, _)| id).collect();
        let Some(gid) = connected.first().copied() else {
            eprintln!("no controller connected; skipping reclaim test");
            return;
        };
        let mut sys = GamepadSystem {
            gilrs,
            slots: vec![GamepadState {
                id: Some(GamepadToken(1)),
                // The ghost pad last spoke half an epoch ago, long before
                // the silence threshold.
                last_event_epoch: 0,
                ..GamepadState::DEFAULT
            }],
            id_to_slot: [(gid, 0)].into_iter().collect(),
            unassigned: Vec::new(),
            assign_mode: GamepadAssignMode::default(),
            epoch: STALE_SLOT_EPOCHS + 10,
        };
        sys.reclaim_stale_slots();
        assert!(
            sys.slots[0].id.is_none(),
            "a silent pad's slot must free up for a replacement"
        );
        assert!(
            sys.id_to_slot.is_empty(),
            "the dead pad's gilrs mapping must be dropped"
        );
    }
}

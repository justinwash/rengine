use std::collections::HashMap;
use std::hash::Hash;

use crate::math::LoopMode;
use crate::renderer::TextureId;

#[derive(Debug, Clone)]
pub struct SpriteSheet {
    pub texture: TextureId,

    pub texture_width: u32,

    pub texture_height: u32,

    pub cell_width: u32,

    pub cell_height: u32,
}

impl SpriteSheet {
    pub fn new(
        texture: TextureId,
        texture_width: u32,
        texture_height: u32,
        cell_width: u32,
        cell_height: u32,
    ) -> Self {
        Self {
            texture,
            texture_width,
            texture_height,
            cell_width,
            cell_height,
        }
    }

    pub fn columns(&self) -> u32 {
        self.texture_width / self.cell_width
    }

    pub fn rows(&self) -> u32 {
        self.texture_height / self.cell_height
    }

    pub fn uv_rect(&self, col: u32, row: u32) -> [f32; 4] {
        let tw = self.texture_width as f32;
        let th = self.texture_height as f32;
        let cw = self.cell_width as f32;
        let ch = self.cell_height as f32;
        [col as f32 * cw / tw, row as f32 * ch / th, cw / tw, ch / th]
    }
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub frames: Vec<(u32, u32)>,

    pub frame_time: f32,

    elapsed: f32,

    current: usize,

    direction: i32,

    loop_mode: LoopMode,

    finished: bool,
}

impl Animation {
    pub fn new(frames: Vec<(u32, u32)>, fps: f32) -> Self {
        assert!(!frames.is_empty(), "Animation requires at least one frame");
        assert!(fps > 0.0, "Animation fps must be greater than zero");

        Self {
            frames,
            frame_time: 1.0 / fps,
            elapsed: 0.0,
            current: 0,
            direction: 1,
            loop_mode: LoopMode::Loop,
            finished: false,
        }
    }

    pub fn once(frames: Vec<(u32, u32)>, fps: f32) -> Self {
        Self::new(frames, fps).with_loop_mode(LoopMode::Once)
    }

    pub fn with_loop_mode(mut self, loop_mode: LoopMode) -> Self {
        self.loop_mode = loop_mode;
        self.reset();
        self
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn update(&mut self, dt: f32) -> (u32, u32) {
        if self.loop_mode == LoopMode::Once && self.finished {
            return self.current_frame();
        }

        self.elapsed += dt;
        let steps = (self.elapsed / self.frame_time) as usize;
        if steps == 0 {
            return self.current_frame();
        }

        self.elapsed = self.elapsed.rem_euclid(self.frame_time);
        if !self.advance_steps(steps) {
            self.elapsed = 0.0;
        }
        self.current_frame()
    }

    pub fn current_frame(&self) -> (u32, u32) {
        self.frames[self.current]
    }

    pub fn reset(&mut self) {
        self.current = 0;
        self.elapsed = 0.0;
        self.direction = 1;
        self.finished = false;
    }

    fn advance_steps(&mut self, steps: usize) -> bool {
        match self.loop_mode {
            LoopMode::Loop => {
                self.current = (self.current + steps % self.frames.len()) % self.frames.len();
                true
            }
            LoopMode::Once => {
                let last_frame = self.frames.len() - 1;
                let remaining = last_frame.saturating_sub(self.current);
                if steps >= remaining {
                    self.current = last_frame;
                    self.finished = true;
                    false
                } else {
                    self.current += steps;
                    true
                }
            }
            LoopMode::PingPong => {
                if self.frames.len() <= 1 {
                    return false;
                }

                let mut remaining = steps;
                let cycle_len = 2 * (self.frames.len() - 1);

                if self.current == 0 && self.direction >= 0 {
                    self.current = 1;
                    self.direction = 1;
                    remaining = remaining.saturating_sub(1);
                    if remaining == 0 {
                        return true;
                    }
                }

                let phase = if self.direction >= 0 {
                    self.current - 1
                } else {
                    cycle_len - 1 - self.current
                };
                let phase = (phase + remaining % cycle_len) % cycle_len;

                if phase < self.frames.len() - 1 {
                    self.current = phase + 1;
                    self.direction = 1;
                } else {
                    self.current = cycle_len - 1 - phase;
                    self.direction = -1;
                }

                true
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnimationState<State> {
    pub animation: Animation,

    pub on_complete: Option<State>,
}

impl<State> AnimationState<State> {
    pub fn new(animation: Animation) -> Self {
        Self {
            animation,
            on_complete: None,
        }
    }

    pub fn with_on_complete(mut self, next_state: State) -> Self {
        self.on_complete = Some(next_state);
        self
    }
}

#[derive(Debug, Clone)]
pub struct AnimationTransition<State> {
    pub target: State,

    pub reset: bool,
}

impl<State> AnimationTransition<State> {
    pub fn new(target: State) -> Self {
        Self {
            target,
            reset: true,
        }
    }

    pub fn preserve_progress(mut self) -> Self {
        self.reset = false;
        self
    }
}

#[derive(Debug, Clone)]
pub struct AnimationStateMachine<State, Trigger>
where
    State: Eq + Hash + Clone,
    Trigger: Eq + Hash + Clone,
{
    states: HashMap<State, AnimationState<State>>,
    transitions: HashMap<State, HashMap<Trigger, AnimationTransition<State>>>,
    global_transitions: HashMap<Trigger, AnimationTransition<State>>,
    current_state: State,
}

impl<State, Trigger> AnimationStateMachine<State, Trigger>
where
    State: Eq + Hash + Clone,
    Trigger: Eq + Hash + Clone,
{
    pub fn new(initial_state: State, animation: Animation) -> Self {
        let mut states = HashMap::new();
        states.insert(initial_state.clone(), AnimationState::new(animation));

        Self {
            states,
            transitions: HashMap::new(),
            global_transitions: HashMap::new(),
            current_state: initial_state,
        }
    }

    pub fn add_state(&mut self, state: State, animation: Animation) -> &mut Self {
        self.states.insert(state, AnimationState::new(animation));
        self
    }

    pub fn add_state_with(
        &mut self,
        state: State,
        animation_state: AnimationState<State>,
    ) -> &mut Self {
        self.states.insert(state, animation_state);
        self
    }

    pub fn set_on_complete(&mut self, state: State, next_state: State) -> &mut Self {
        if let Some(animation_state) = self.states.get_mut(&state) {
            animation_state.on_complete = Some(next_state);
        }
        self
    }

    pub fn add_transition(&mut self, from: State, trigger: Trigger, to: State) -> &mut Self {
        self.add_transition_with(from, trigger, AnimationTransition::new(to))
    }

    pub fn add_transition_with(
        &mut self,
        from: State,
        trigger: Trigger,
        transition: AnimationTransition<State>,
    ) -> &mut Self {
        self.transitions
            .entry(from)
            .or_default()
            .insert(trigger, transition);
        self
    }

    pub fn add_global_transition(&mut self, trigger: Trigger, to: State) -> &mut Self {
        self.add_global_transition_with(trigger, AnimationTransition::new(to))
    }

    pub fn add_global_transition_with(
        &mut self,
        trigger: Trigger,
        transition: AnimationTransition<State>,
    ) -> &mut Self {
        self.global_transitions.insert(trigger, transition);
        self
    }

    pub fn current_state(&self) -> &State {
        &self.current_state
    }

    pub fn current_frame(&self) -> (u32, u32) {
        self.animation().current_frame()
    }

    pub fn current_uv_rect(&self, sprite_sheet: &SpriteSheet) -> [f32; 4] {
        let (col, row) = self.current_frame();
        sprite_sheet.uv_rect(col, row)
    }

    pub fn animation(&self) -> &Animation {
        &self
            .states
            .get(&self.current_state)
            .expect("AnimationStateMachine missing current state")
            .animation
    }

    pub fn animation_mut(&mut self) -> &mut Animation {
        &mut self
            .states
            .get_mut(&self.current_state)
            .expect("AnimationStateMachine missing current state")
            .animation
    }

    pub fn is_finished(&self) -> bool {
        self.animation().is_finished()
    }

    pub fn set_state(&mut self, state: State) -> bool {
        if !self.states.contains_key(&state) {
            return false;
        }

        self.current_state = state.clone();
        self.states
            .get_mut(&state)
            .expect("AnimationStateMachine missing target state")
            .animation
            .reset();
        true
    }

    pub fn trigger(&mut self, trigger: Trigger) -> bool {
        let transition = self
            .transitions
            .get(&self.current_state)
            .and_then(|transitions| transitions.get(&trigger).cloned())
            .or_else(|| self.global_transitions.get(&trigger).cloned());

        if let Some(transition) = transition {
            self.apply_transition(transition)
        } else {
            false
        }
    }

    pub fn update(&mut self, dt: f32) -> (u32, u32) {
        let current_state = self.current_state.clone();
        let (frame, next_state) = {
            let animation_state = self
                .states
                .get_mut(&current_state)
                .expect("AnimationStateMachine missing current state");
            let frame = animation_state.animation.update(dt);
            let next_state = if animation_state.animation.is_finished() {
                animation_state.on_complete.clone()
            } else {
                None
            };
            (frame, next_state)
        };

        if let Some(next_state) = next_state {
            self.apply_transition(AnimationTransition::new(next_state));
            self.current_frame()
        } else {
            frame
        }
    }

    fn apply_transition(&mut self, transition: AnimationTransition<State>) -> bool {
        if !self.states.contains_key(&transition.target) {
            log::warn!("AnimationStateMachine ignoring transition to missing state");
            return false;
        }

        let should_reset = transition.reset || transition.target != self.current_state;
        self.current_state = transition.target.clone();

        if should_reset {
            self.states
                .get_mut(&transition.target)
                .expect("AnimationStateMachine missing target state")
                .animation
                .reset();
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    enum TestState {
        Idle,
        Hit,
        Recover,
        Missing,
    }

    #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    enum TestTrigger {
        Hit,
    }

    #[test]
    fn once_animation_stops_on_last_frame() {
        let mut animation = Animation::once(vec![(0, 0), (1, 0)], 10.0);

        assert_eq!(animation.update(0.1), (1, 0));
        assert!(animation.is_finished());
        assert_eq!(animation.update(0.5), (1, 0));
    }

    #[test]
    fn ping_pong_animation_reverses_direction() {
        let mut animation =
            Animation::new(vec![(0, 0), (1, 0), (2, 0)], 10.0).with_loop_mode(LoopMode::PingPong);

        assert_eq!(animation.update(0.1), (1, 0));
        assert_eq!(animation.update(0.1), (2, 0));
        assert_eq!(animation.update(0.1), (1, 0));
        assert_eq!(animation.update(0.1), (0, 0));
    }

    #[test]
    fn loop_animation_skips_large_dt() {
        let mut animation = Animation::new(vec![(0, 0), (1, 0), (2, 0)], 10.0);

        assert_eq!(animation.update(0.85), (2, 0));
    }

    #[test]
    fn state_machine_uses_trigger_and_completion_transition() {
        let mut machine =
            AnimationStateMachine::new(TestState::Idle, Animation::new(vec![(0, 0)], 4.0));
        machine.add_state_with(
            TestState::Hit,
            AnimationState::new(Animation::once(vec![(1, 0), (2, 0)], 10.0))
                .with_on_complete(TestState::Recover),
        );
        machine.add_state(TestState::Recover, Animation::new(vec![(3, 0)], 4.0));
        machine.add_transition(TestState::Idle, TestTrigger::Hit, TestState::Hit);

        assert!(machine.trigger(TestTrigger::Hit));
        assert_eq!(machine.current_state(), &TestState::Hit);

        machine.update(0.1);

        assert_eq!(machine.current_state(), &TestState::Recover);
        assert_eq!(machine.current_frame(), (3, 0));
    }

    #[test]
    fn missing_transition_target_is_ignored() {
        let mut machine =
            AnimationStateMachine::new(TestState::Idle, Animation::new(vec![(0, 0)], 4.0));
        machine.add_global_transition(TestTrigger::Hit, TestState::Missing);

        assert!(!machine.trigger(TestTrigger::Hit));
        assert_eq!(machine.current_state(), &TestState::Idle);
    }

    #[test]
    fn missing_on_complete_target_is_ignored() {
        let mut machine: AnimationStateMachine<TestState, TestTrigger> = AnimationStateMachine::new(
            TestState::Idle,
            Animation::once(vec![(0, 0), (1, 0)], 10.0),
        );
        machine.set_on_complete(TestState::Idle, TestState::Missing);

        machine.update(0.1);

        assert_eq!(machine.current_state(), &TestState::Idle);
        assert_eq!(machine.current_frame(), (1, 0));
        assert!(machine.is_finished());
    }
}

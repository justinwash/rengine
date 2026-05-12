mod data2d;
mod globals;
mod script2d;

pub use data2d::{
    Prefab2D, Prefab2DDef, PrefabSprite2D, PrefabSprite2DDef, Scene2D, Scene2DDef, SceneInstance2D,
    SceneInstance2DDef, SceneScriptBinding2D,
};

pub use globals::Globals;
pub use script2d::{
    SceneScript2D, SceneScriptEvent2D, SceneScriptHost2D, SceneScriptInputEvent2D,
    SceneScriptRegistry2D,
};

use crate::app::{Engine, Engine3D};
use crate::assets::Color;
use crate::renderer::Frame;
use crate::renderer3d::Frame3D;

#[derive(Clone, Copy)]
pub struct Transition {
    pub color: Color,
    pub duration: f32,
}

impl Transition {
    pub fn fade(duration: f32) -> Self {
        Self {
            color: Color::BLACK,
            duration,
        }
    }

    pub fn fade_color(color: Color, duration: f32) -> Self {
        Self { color, duration }
    }

    pub fn fade_white(duration: f32) -> Self {
        Self {
            color: Color::WHITE,
            duration,
        }
    }
}

pub enum SceneOp {
    Continue,
    Push(Box<dyn Scene>),
    Switch(Box<dyn Scene>),
    Pop,
    Quit,
    FadePush(Box<dyn Scene>, Transition),
    FadeSwitch(Box<dyn Scene>, Transition),
    FadePop(Transition),
}

pub(crate) struct ActiveTransition {
    pub color: Color,
    pub duration: f32,
    pub elapsed: f32,
    pub pending_op: Option<SceneOp>,
}

impl ActiveTransition {
    pub fn new(transition: Transition, pending_op: SceneOp) -> Self {
        Self {
            color: transition.color,
            duration: transition.duration,
            elapsed: 0.0,
            pending_op: Some(pending_op),
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.elapsed += dt;
    }

    pub fn is_done(&self) -> bool {
        self.elapsed >= self.duration
    }

    pub fn at_midpoint(&self) -> bool {
        self.elapsed >= self.duration / 2.0 && self.pending_op.is_some()
    }

    pub fn alpha(&self) -> f32 {
        let half = self.duration / 2.0;
        if half <= 0.0 {
            return 0.0;
        }
        if self.elapsed < half {
            (self.elapsed / half).clamp(0.0, 1.0)
        } else {
            (1.0 - (self.elapsed - half) / half).clamp(0.0, 1.0)
        }
    }
}

pub trait Scene: 'static {
    fn on_enter(&mut self, engine: &mut Engine, globals: &mut Globals);

    fn update(&mut self, engine: &Engine, globals: &mut Globals, frame: &mut Frame) -> SceneOp;

    fn fixed_update(&mut self, _engine: &Engine, _globals: &mut Globals) {}

    fn render(&self, _engine: &Engine, _globals: &Globals, _frame: &mut Frame) {}

    fn on_pause(&mut self, _engine: &Engine, _globals: &Globals) {}

    fn on_resume(&mut self, _engine: &mut Engine, _globals: &mut Globals) {}

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {}
}

pub enum SceneOp3D {
    Continue,
    Push(Box<dyn Scene3D>),
    Switch(Box<dyn Scene3D>),
    Pop,
    Quit,
}

pub trait Scene3D: 'static {
    fn on_enter(&mut self, engine: &mut Engine3D, globals: &mut Globals);
    fn update(
        &mut self,
        engine: &Engine3D,
        globals: &mut Globals,
        frame: &mut Frame3D,
    ) -> SceneOp3D;
    fn fixed_update(&mut self, _engine: &Engine3D, _globals: &mut Globals) {}
    fn render(&self, _engine: &Engine3D, _globals: &Globals, _frame: &mut Frame3D) {}
    fn on_pause(&mut self, _engine: &Engine3D, _globals: &Globals) {}
    fn on_resume(&mut self, _engine: &mut Engine3D, _globals: &mut Globals) {}
    fn on_exit(&mut self, _engine: &Engine3D, _globals: &Globals) {}
}

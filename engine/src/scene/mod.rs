mod data2d;
mod globals;

pub use data2d::{
    Prefab2D, Prefab2DDef, PrefabSprite2D, PrefabSprite2DDef, Scene2D, Scene2DDef, SceneInstance2D,
    SceneInstance2DDef,
};

pub use globals::Globals;

use crate::app::{Engine, Engine3D};
use crate::renderer::Frame;
use crate::renderer3d::Frame3D;


pub enum SceneOp {
    Continue,
    Push(Box<dyn Scene>),
    Switch(Box<dyn Scene>),
    Pop,
    Quit,
}

pub trait Scene: 'static {
    fn on_enter(&mut self, engine: &mut Engine, globals: &mut Globals);

    fn update(&mut self, engine: &Engine, globals: &mut Globals) -> SceneOp;

    fn fixed_update(&mut self, _engine: &Engine, _globals: &mut Globals) {}

    fn render(&self, engine: &Engine, globals: &Globals, frame: &mut Frame);

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
    fn update(&mut self, engine: &Engine3D, globals: &mut Globals) -> SceneOp3D;
    fn fixed_update(&mut self, _engine: &Engine3D, _globals: &mut Globals) {}
    fn render(&self, engine: &Engine3D, globals: &Globals, frame: &mut Frame3D);
    fn on_pause(&mut self, _engine: &Engine3D, _globals: &Globals) {}
    fn on_resume(&mut self, _engine: &mut Engine3D, _globals: &mut Globals) {}
    fn on_exit(&mut self, _engine: &Engine3D, _globals: &Globals) {}
}

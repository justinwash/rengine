use crate::game_object::actor::Actor;
use crate::game_object::GameObject;
use rapier2d::geometry::BroadPhaseBvh as BroadPhase;
use rapier2d::geometry::NarrowPhase;
use rapier2d::pipeline::QueryPipeline;
use rapier2d::prelude::*;

pub struct RigidBodyActor {
    pub rigid_body: RigidBodyHandle,
    pub collider: ColliderHandle,
}

impl RigidBodyActor {
    pub fn new(position: (f32, f32)) -> Self {
        let rigid_body = RigidBodyHandle::default(); // Placeholder for actual initialization
        let collider = ColliderHandle::default(); // Placeholder for actual initialization
        Self {
            rigid_body,
            collider,
        }
    }

    pub fn update_position(&mut self, pos: (f32, f32)) {
        // Placeholder for updating position logic
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }
}

impl GameObject for RigidBodyActor {
    fn position(&self) -> (f32, f32) {
        // Placeholder for getting position logic
        (0.0, 0.0)
    }

    fn set_position(&mut self, pos: (f32, f32)) {
        // Placeholder for setting position logic
    }
}

impl Actor for RigidBodyActor {
    fn update(
        &mut self,
        _wgpu_ctx: &mut crate::window::WgpuContext,
        _input: &crate::input::InputState,
        _event: &winit::event::Event<()>,
        _window: &winit::window::Window,
    ) {
        // Physics update logic can be added here
    }

    fn draw(
        &mut self,
        _renderer: &mut crate::sprite::SpriteRenderer,
        _wgpu_ctx: &mut crate::window::WgpuContext,
    ) {
        // Drawing logic can be added here
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }
}

pub fn create_dynamic_rigid_body() -> RigidBodyBuilder {
    RigidBodyBuilder::new(RigidBodyType::Dynamic)
}

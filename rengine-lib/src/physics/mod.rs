use rapier2d::pipeline::PhysicsPipeline;
use rapier2d::prelude::{BroadPhaseBvh, *};

pub struct PhysicsWorld {
    pub gravity: Vector<f32>,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub island_manager: IslandManager,
    pub narrow_phase: NarrowPhase,
    pub ccd_solver: CCDSolver,
    pub broad_phase: BroadPhaseBvh,
}

impl PhysicsWorld {
    pub fn new(gravity: Vector<f32>) -> Self {
        Self {
            gravity,
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            island_manager: IslandManager::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
            broad_phase: BroadPhaseBvh::new(),
        }
    }

    pub fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(), // No hooks for now
            &(), // No events for now
        );
    }

    pub fn add_rigid_body(&mut self, body: RigidBody) -> RigidBodyHandle {
        self.bodies.insert(body)
    }

    pub fn remove_rigid_body(&mut self, handle: RigidBodyHandle) {
        self.bodies.remove(
            handle,
            &mut self.island_manager,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true, // Wake up other bodies
        );
    }

    pub fn add_collider(&mut self, collider: Collider, parent: RigidBodyHandle) -> ColliderHandle {
        self.colliders
            .insert_with_parent(collider, parent, &mut self.bodies)
    }

    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        self.colliders.remove(
            handle,
            &mut self.island_manager,
            &mut self.bodies,
            true, // Wake up other bodies
        );
    }
}

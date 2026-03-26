use rengine::*;

use crate::state::{CollisionWall, DoorDef};
use crate::WALL_HEIGHT;

pub struct LevelBuilder {
    pub verts: Vec<Vertex3D>,
    pub idxs: Vec<u32>,
}

impl LevelBuilder {
    pub fn new() -> Self {
        Self {
            verts: Vec::new(),
            idxs: Vec::new(),
        }
    }

    pub fn wall(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, y: f32, h: f32, color: Color) {
        let (v, i) = wall_quad([x0, y, z0], [x1, y, z1], h, color);
        let base = self.verts.len() as u32;
        self.verts.extend_from_slice(&v);
        self.idxs.extend(i.iter().map(|idx| idx + base));
    }

    pub fn floor_rect(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, y: f32, color: Color) {
        let base = self.verts.len() as u32;
        let n = [0.0, 1.0, 0.0];
        let c = color.to_array();
        self.verts.push(Vertex3D {
            position: [x0, y, z0],
            normal: n,
            color: c,
        });
        self.verts.push(Vertex3D {
            position: [x1, y, z0],
            normal: n,
            color: c,
        });
        self.verts.push(Vertex3D {
            position: [x1, y, z1],
            normal: n,
            color: c,
        });
        self.verts.push(Vertex3D {
            position: [x0, y, z1],
            normal: n,
            color: c,
        });
        self.idxs
            .extend_from_slice(&[base + 2, base + 1, base, base, base + 3, base + 2]);
    }

    pub fn ceiling_rect(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, y: f32, color: Color) {
        let base = self.verts.len() as u32;
        let n = [0.0, -1.0, 0.0];
        let c = color.to_array();
        self.verts.push(Vertex3D {
            position: [x0, y, z1],
            normal: n,
            color: c,
        });
        self.verts.push(Vertex3D {
            position: [x1, y, z1],
            normal: n,
            color: c,
        });
        self.verts.push(Vertex3D {
            position: [x1, y, z0],
            normal: n,
            color: c,
        });
        self.verts.push(Vertex3D {
            position: [x0, y, z0],
            normal: n,
            color: c,
        });
        self.idxs
            .extend_from_slice(&[base + 2, base + 1, base, base, base + 3, base + 2]);
    }
}

pub fn build(engine: &mut Engine3D) -> BuildResult {
    let mut builder = LevelBuilder::new();

    let floor_col = Color::from_rgba8(100, 100, 100, 255);
    let wall_col = Color::from_rgba8(160, 150, 140, 255);
    let wall_col2 = Color::from_rgba8(140, 130, 120, 255);
    let ceil_col = Color::from_rgba8(80, 80, 90, 255);
    let accent = Color::from_rgba8(120, 60, 40, 255);
    let floor_col2 = Color::from_rgba8(120, 110, 100, 255);

    builder.floor_rect(0.0, 0.0, 8.0, 8.0, 0.0, floor_col);
    builder.ceiling_rect(0.0, 0.0, 8.0, 8.0, WALL_HEIGHT, ceil_col);
    builder.wall(0.0, 0.0, 8.0, 0.0, 0.0, WALL_HEIGHT, wall_col);
    builder.wall(8.0, 8.0, 0.0, 8.0, 0.0, WALL_HEIGHT, wall_col);
    builder.wall(0.0, 8.0, 0.0, 0.0, 0.0, WALL_HEIGHT, wall_col);
    builder.wall(8.0, 0.0, 8.0, 3.0, 0.0, WALL_HEIGHT, wall_col);
    builder.wall(8.0, 5.0, 8.0, 8.0, 0.0, WALL_HEIGHT, wall_col);
    builder.wall(8.0, 3.0, 8.0, 5.0, 2.2, WALL_HEIGHT - 2.2, wall_col);

    builder.floor_rect(8.0, 2.0, 16.0, 6.0, 0.0, floor_col2);
    builder.ceiling_rect(8.0, 2.0, 16.0, 6.0, WALL_HEIGHT, ceil_col);
    builder.wall(8.0, 2.0, 16.0, 2.0, 0.0, WALL_HEIGHT, wall_col2);
    builder.wall(16.0, 6.0, 8.0, 6.0, 0.0, WALL_HEIGHT, wall_col2);
    builder.wall(16.0, 2.0, 16.0, 3.0, 0.0, WALL_HEIGHT, wall_col2);
    builder.wall(16.0, 5.0, 16.0, 6.0, 0.0, WALL_HEIGHT, wall_col2);
    builder.wall(16.0, 3.0, 16.0, 5.0, 2.2, WALL_HEIGHT - 2.2, wall_col2);

    builder.floor_rect(16.0, 0.0, 28.0, 12.0, 0.0, floor_col);
    builder.ceiling_rect(16.0, 0.0, 28.0, 12.0, WALL_HEIGHT, ceil_col);
    builder.wall(16.0, 0.0, 28.0, 0.0, 0.0, WALL_HEIGHT, accent);
    builder.wall(28.0, 12.0, 16.0, 12.0, 0.0, WALL_HEIGHT, accent);
    builder.wall(28.0, 0.0, 28.0, 12.0, 0.0, WALL_HEIGHT, accent);
    builder.wall(16.0, 12.0, 16.0, 6.0, 0.0, WALL_HEIGHT, wall_col2);
    builder.wall(16.0, 2.0, 16.0, 0.0, 0.0, WALL_HEIGHT, wall_col2);

    let pillar_col = Color::from_rgba8(90, 80, 70, 255);
    for &(px, pz) in &[(20.0, 3.0), (20.0, 9.0), (24.0, 3.0), (24.0, 9.0)] {
        let (v, i) = cube_mesh(0.6, WALL_HEIGHT, 0.6, pillar_col);
        let base = builder.verts.len() as u32;
        for mut vert in v {
            vert.position[0] += px;
            vert.position[1] += WALL_HEIGHT / 2.0;
            vert.position[2] += pz;
            builder.verts.push(vert);
        }
        builder.idxs.extend(i.iter().map(|idx| idx + base));
    }

    let mut walls = vec![
        CollisionWall::new(0.0, 0.0, 8.0, 0.0),
        CollisionWall::new(0.0, 8.0, 8.0, 8.0),
        CollisionWall::new(0.0, 0.0, 0.0, 8.0),
        CollisionWall::new(8.0, 0.0, 8.0, 3.0),
        CollisionWall::new(8.0, 5.0, 8.0, 8.0),
        CollisionWall::new(8.0, 2.0, 16.0, 2.0),
        CollisionWall::new(8.0, 6.0, 16.0, 6.0),
        CollisionWall::new(16.0, 2.0, 16.0, 3.0),
        CollisionWall::new(16.0, 5.0, 16.0, 6.0),
        CollisionWall::new(16.0, 0.0, 28.0, 0.0),
        CollisionWall::new(16.0, 12.0, 28.0, 12.0),
        CollisionWall::new(28.0, 0.0, 28.0, 12.0),
        CollisionWall::new(16.0, 6.0, 16.0, 12.0),
        CollisionWall::new(16.0, 0.0, 16.0, 2.0),
    ];

    for &(px, pz) in &[(20.0, 3.0), (20.0, 9.0), (24.0, 3.0), (24.0, 9.0)] {
        let half = 0.3;
        walls.push(CollisionWall::new(
            px - half,
            pz - half,
            px + half,
            pz - half,
        ));
        walls.push(CollisionWall::new(
            px - half,
            pz + half,
            px + half,
            pz + half,
        ));
        walls.push(CollisionWall::new(
            px - half,
            pz - half,
            px - half,
            pz + half,
        ));
        walls.push(CollisionWall::new(
            px + half,
            pz - half,
            px + half,
            pz + half,
        ));
    }

    let door_color = Color::from_rgba8(139, 90, 43, 255);
    let (dv, di) = cube_mesh(0.15, 2.2, 2.0, door_color);
    let door_mesh1 = engine.create_mesh(dv.clone(), di.clone());
    let door_mesh2 = engine.create_mesh(dv, di);

    let door_defs = vec![
        DoorDef {
            x: 8.0,
            z: 4.0,
            slides_x: false,
            trigger_radius: 2.0,
            wall: CollisionWall::new(8.0, 3.0, 8.0, 5.0),
        },
        DoorDef {
            x: 16.0,
            z: 4.0,
            slides_x: false,
            trigger_radius: 2.0,
            wall: CollisionWall::new(16.0, 3.0, 16.0, 5.0),
        },
    ];

    let player_color = Color::from_rgba8(50, 150, 220, 255);
    let (pv, pi) = cube_mesh(0.6, 1.7, 0.6, player_color);
    let player_mesh = engine.create_mesh(pv, pi);

    let projectile_color = Color::YELLOW;
    let (bv, bi) = cube_mesh(0.1, 0.1, 0.3, projectile_color);
    let projectile_mesh = engine.create_mesh(bv, bi);

    let spawn_points = [
        [4.0, crate::PLAYER_HEIGHT, 4.0],
        [24.0, crate::PLAYER_HEIGHT, 6.0],
    ];

    BuildResult {
        level_verts: builder.verts,
        level_idxs: builder.idxs,
        walls,
        door_defs,
        door_meshes: vec![door_mesh1, door_mesh2],
        player_mesh,
        projectile_mesh,
        spawn_points,
    }
}

pub struct BuildResult {
    pub level_verts: Vec<Vertex3D>,
    pub level_idxs: Vec<u32>,
    pub walls: Vec<CollisionWall>,
    pub door_defs: Vec<DoorDef>,
    pub door_meshes: Vec<MeshId>,
    pub player_mesh: MeshId,
    pub projectile_mesh: MeshId,
    pub spawn_points: [[f32; 3]; 2],
}

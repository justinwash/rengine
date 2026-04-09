use std::path::PathBuf;

use rengine::*;

use crate::state::{CollisionWall, Enemy, FpsGame};
use crate::{PLAYER_HEIGHT, WALL_HEIGHT};

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

    #[allow(dead_code)]
    pub fn room(
        &mut self,
        x0: f32,
        z0: f32,
        x1: f32,
        z1: f32,
        floor_color: Color,
        wall_color: Color,
        ceiling_color: Color,
    ) {
        self.floor_rect(x0, z0, x1, z1, 0.0, floor_color);
        self.ceiling_rect(x0, z0, x1, z1, WALL_HEIGHT, ceiling_color);
        self.wall(x0, z0, x1, z0, 0.0, WALL_HEIGHT, wall_color);
        self.wall(x1, z1, x0, z1, 0.0, WALL_HEIGHT, wall_color);
        self.wall(x0, z1, x0, z0, 0.0, WALL_HEIGHT, wall_color);
        self.wall(x1, z0, x1, z1, 0.0, WALL_HEIGHT, wall_color);
    }
}

pub fn build(engine: &mut Engine3D) -> FpsGame {
    engine.set_asset_root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
    let assets = engine
        .load_asset_manifest("fps.assets.json")
        .expect("failed to load FPS asset manifest");

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

    let level_verts = builder.verts;
    let level_idxs = builder.idxs;

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

    let doors = Vec::new();

    let enemy_mesh = assets
        .mesh("enemy")
        .expect("manifest missing enemy mesh")
        .mesh();
    let mut enemies = Vec::new();
    let enemy_positions = vec![
        Vec3::new(12.0, 0.0, 4.0),
        Vec3::new(20.0, 0.0, 6.0),
        Vec3::new(24.0, 0.0, 6.0),
        Vec3::new(22.0, 0.0, 1.5),
        Vec3::new(22.0, 0.0, 10.5),
        Vec3::new(26.0, 0.0, 6.0),
    ];
    for pos in enemy_positions {
        enemies.push(Enemy {
            pos,
            alive: true,
            mesh: enemy_mesh,
        });
    }

    let projectile_mesh = assets
        .mesh("projectile")
        .expect("manifest missing projectile mesh")
        .mesh();
    let shoot_sfx = assets
        .audio("shoot")
        .expect("manifest missing shoot audio")
        .clone();
    let hit_sfx = assets
        .audio("hit")
        .expect("manifest missing hit audio")
        .clone();
    let jump_sfx = assets
        .audio("jump")
        .expect("manifest missing jump audio")
        .clone();
    let viewmodel_mesh = build_viewmodel_mesh(engine);

    FpsGame {
        level_verts,
        level_idxs,
        walls,
        doors,
        cam_yaw: 0.0,
        cam_pitch: 0.0,
        player_pos: Vec3::new(4.0, PLAYER_HEIGHT, 4.0),
        player_vel_y: 0.0,
        on_ground: true,
        projectiles: Vec::new(),
        next_projectile_pair_id: 1,
        projectile_mesh,
        viewmodel_mesh,
        enemies,
        score: 0,
        shoot_sfx,
        hit_sfx,
        jump_sfx,
    }
}

fn build_viewmodel_mesh(engine: &mut Engine3D) -> MeshId {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    append_box(
        &mut vertices,
        &mut indices,
        Vec3::new(0.34, -0.24, -0.78),
        Vec3::new(0.18, 0.12, 0.38),
        Color::from_rgba8(80, 82, 90, 255),
    );
    append_box(
        &mut vertices,
        &mut indices,
        Vec3::new(0.40, -0.21, -1.02),
        Vec3::new(0.05, 0.04, 0.34),
        Color::from_rgba8(110, 112, 122, 255),
    );
    append_box(
        &mut vertices,
        &mut indices,
        Vec3::new(0.25, -0.34, -0.63),
        Vec3::new(0.07, 0.18, 0.12),
        Color::from_rgba8(55, 57, 63, 255),
    );
    append_box(
        &mut vertices,
        &mut indices,
        Vec3::new(0.33, -0.17, -0.67),
        Vec3::new(0.10, 0.03, 0.10),
        Color::from_rgba8(180, 110, 45, 255),
    );

    engine.create_mesh(vertices, indices)
}

fn append_box(
    vertices: &mut Vec<Vertex3D>,
    indices: &mut Vec<u32>,
    center: Vec3,
    size: Vec3,
    color: Color,
) {
    let (mut verts, idxs) = cube_mesh(size.x, size.y, size.z, color);
    let base = vertices.len() as u32;
    for vertex in &mut verts {
        vertex.position[0] += center.x;
        vertex.position[1] += center.y;
        vertex.position[2] += center.z;
    }
    vertices.extend(verts);
    indices.extend(idxs.into_iter().map(|index| base + index));
}

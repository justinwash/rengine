use rengine::*;

use crate::state::{CollisionWall, Door, Enemy, FpsGame};
use crate::{ENEMY_SIZE, PLAYER_HEIGHT, WALL_HEIGHT};


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


    let door_color = Color::from_rgba8(139, 90, 43, 255);
    let (dv, di) = cube_mesh(0.15, 2.2, 2.0, door_color);
    let door_mesh1 = engine.create_mesh(dv.clone(), di.clone());
    let door_mesh2 = engine.create_mesh(dv, di);

    let doors = vec![
        Door {
            x: 8.0,
            z: 4.0,
            slides_x: false,
            offset: 0.0,
            open: false,
            mesh: door_mesh1,
            trigger_radius: 2.0,
            wall: CollisionWall::new(8.0, 3.0, 8.0, 5.0),
        },
        Door {
            x: 16.0,
            z: 4.0,
            slides_x: false,
            offset: 0.0,
            open: false,
            mesh: door_mesh2,
            trigger_radius: 2.0,
            wall: CollisionWall::new(16.0, 3.0, 16.0, 5.0),
        },
    ];


    let enemy_color = Color::from_rgba8(200, 50, 50, 255);
    let mut enemies = Vec::new();
    let enemy_positions = vec![
        Vec3::new(12.0, ENEMY_SIZE / 2.0, 4.0),
        Vec3::new(20.0, ENEMY_SIZE / 2.0, 6.0),
        Vec3::new(24.0, ENEMY_SIZE / 2.0, 6.0),
        Vec3::new(22.0, ENEMY_SIZE / 2.0, 1.5),
        Vec3::new(22.0, ENEMY_SIZE / 2.0, 10.5),
        Vec3::new(26.0, ENEMY_SIZE / 2.0, 6.0),
    ];
    for pos in enemy_positions {
        let (ev, ei) = cube_mesh(ENEMY_SIZE, ENEMY_SIZE, ENEMY_SIZE, enemy_color);
        let mesh = engine.create_mesh(ev, ei);
        enemies.push(Enemy {
            pos,
            alive: true,
            mesh,
        });
    }


    let (pv, pi) = cube_mesh(0.1, 0.1, 0.3, Color::YELLOW);
    let projectile_mesh = engine.create_mesh(pv, pi);

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
        projectile_mesh,
        enemies,
        score: 0,
    }
}

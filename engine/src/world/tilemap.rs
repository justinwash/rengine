use glam::Vec2;
use crate::assets::Color;
use crate::renderer::{DrawParams, Frame, TextureId};

pub struct TileMap {
    pub width: usize,
    pub height: usize,
    pub tile_size: f32,
    cells: Vec<Option<usize>>,
    tiles: Vec<TileDef>,
}

#[derive(Clone)]
pub struct TileDef {
    pub texture: TextureId,
    pub color: Color,

    pub uv_rect: [f32; 4],
}

impl TileDef {
    pub fn solid(texture: TextureId) -> Self {
        Self {
            texture,
            color: Color::WHITE,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
        }
    }

    pub fn colored(texture: TextureId, color: Color) -> Self {
        Self {
            texture,
            color,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
        }
    }

    pub fn with_uv(mut self, uv: [f32; 4]) -> Self {
        self.uv_rect = uv;
        self
    }
}

impl TileMap {

    pub fn new(width: usize, height: usize, tile_size: f32) -> Self {
        Self {
            width,
            height,
            tile_size,
            cells: vec![None; width * height],
            tiles: Vec::new(),
        }
    }

    pub fn add_tile(&mut self, def: TileDef) -> usize {
        let id = self.tiles.len();
        self.tiles.push(def);
        id
    }

    pub fn set(&mut self, col: usize, row: usize, tile: Option<usize>) {
        if col < self.width && row < self.height {
            self.cells[row * self.width + col] = tile;
        }
    }

    pub fn get(&self, col: usize, row: usize) -> Option<usize> {
        if col < self.width && row < self.height {
            self.cells[row * self.width + col]
        } else {
            None
        }
    }

    pub fn cell_position(&self, col: usize, row: usize) -> Vec2 {
        Vec2::new(col as f32 * self.tile_size, row as f32 * self.tile_size)
    }

    pub fn world_width(&self) -> f32 {
        self.width as f32 * self.tile_size
    }

    pub fn world_height(&self) -> f32 {
        self.height as f32 * self.tile_size
    }

    pub fn collide_rect(&self, rect: &crate::math::Rect) -> Option<Vec2> {
        use super::physics::aabb_overlap;
        use crate::math::rect::Rect;

        let col_min = ((rect.x / self.tile_size).floor() as isize).max(0) as usize;
        let col_max = (((rect.x + rect.width) / self.tile_size).ceil() as usize).min(self.width);
        let row_min = ((rect.y / self.tile_size).floor() as isize).max(0) as usize;
        let row_max = (((rect.y + rect.height) / self.tile_size).ceil() as usize).min(self.height);

        let mut total_mtv = Vec2::ZERO;
        let mut collided = false;

        for row in row_min..row_max {
            for col in col_min..col_max {
                if self.cells[row * self.width + col].is_some() {
                    let tile_rect = Rect::new(
                        col as f32 * self.tile_size,
                        row as f32 * self.tile_size,
                        self.tile_size,
                        self.tile_size,
                    );

                    let adjusted = Rect::new(
                        rect.x + total_mtv.x,
                        rect.y + total_mtv.y,
                        rect.width,
                        rect.height,
                    );
                    if let Some(mtv) = aabb_overlap(&adjusted, &tile_rect) {
                        total_mtv += mtv;
                        collided = true;
                    }
                }
            }
        }

        if collided {
            Some(total_mtv)
        } else {
            None
        }
    }

    pub fn draw(&self, frame: &mut Frame) {
        let cam = &frame.camera;

        let half_w = 600.0;
        let half_h = 400.0;
        let left = ((cam.position.x - half_w) / self.tile_size).floor().max(0.0) as usize;
        let right = ((cam.position.x + half_w) / self.tile_size).ceil() as usize;
        let bottom = ((cam.position.y - half_h) / self.tile_size).floor().max(0.0) as usize;
        let top = ((cam.position.y + half_h) / self.tile_size).ceil() as usize;

        let right = right.min(self.width);
        let top = top.min(self.height);

        let size = Vec2::new(self.tile_size, self.tile_size);

        for row in bottom..top {
            for col in left..right {
                if let Some(tile_id) = self.cells[row * self.width + col] {
                    let def = &self.tiles[tile_id];
                    let pos = Vec2::new(col as f32 * self.tile_size, row as f32 * self.tile_size);
                    frame.draw_sprite(
                        DrawParams::new(def.texture, pos, size)
                            .with_color(def.color)
                            .with_uv_rect(def.uv_rect),
                    );
                }
            }
        }
    }
}

use rengine::pixelart::PixelCanvas;
use rengine::*;

const TILE_SIZE: f32 = 32.0;
const MAP_W: usize = 20;
const MAP_H: usize = 15;
const PLAYER_SIZE: f32 = 24.0;
const PLAYER_SPEED: f32 = 160.0;

struct TileMapDemo {
    map: TileMap,
    floor_map: TileMap,
    player_tex: TextureId,
    player_pos: Vec2,
}

fn make_wall_texture(engine: &mut Engine) -> TextureId {
    let mut pc = PixelCanvas::new(16, 16);
    pc.fill(Color::new(0.4, 0.35, 0.3, 1.0));
    for y in 0..16 {
        for x in 0..16 {
            if y % 8 == 0 || (x + (y / 8) * 4) % 8 == 0 {
                pc.set(x, y, Color::new(0.3, 0.25, 0.2, 1.0));
            }
        }
    }
    let bytes = pc.into_bytes();
    engine.create_texture(16, 16, &bytes)
}

fn make_floor_texture(engine: &mut Engine) -> TextureId {
    let mut pc = PixelCanvas::new(16, 16);
    pc.fill(Color::new(0.55, 0.65, 0.5, 1.0));
    for y in (0..16).step_by(4) {
        for x in (0..16).step_by(4) {
            pc.set(x, y, Color::new(0.5, 0.6, 0.45, 1.0));
        }
    }
    let bytes = pc.into_bytes();
    engine.create_texture(16, 16, &bytes)
}

fn make_player_texture(engine: &mut Engine) -> TextureId {
    let mut pc = PixelCanvas::new(12, 12);
    pc.fill_circle(6, 6, 5, Color::new(0.3, 0.6, 1.0, 1.0));
    pc.fill_circle(4, 5, 1, Color::WHITE);
    pc.fill_circle(8, 5, 1, Color::WHITE);
    let bytes = pc.into_bytes();
    engine.create_texture(12, 12, &bytes)
}

#[rustfmt::skip]
const LEVEL: [u8; MAP_W * MAP_H] = [
    1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
    1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,
    1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,
    1,0,0,1,1,1,0,0,0,0,0,0,0,1,1,1,0,0,0,1,
    1,0,0,1,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,1,
    1,0,0,1,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,1,
    1,0,0,0,0,0,0,0,1,1,0,0,0,0,0,0,0,0,0,1,
    1,0,0,0,0,0,0,0,1,1,0,0,0,0,0,0,0,0,0,1,
    1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,
    1,0,0,1,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,1,
    1,0,0,1,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,1,
    1,0,0,1,1,1,0,0,0,0,0,0,0,1,1,1,0,0,0,1,
    1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,
    1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,
    1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
];

impl Game for TileMapDemo {
    fn new(engine: &mut Engine) -> Self {
        let wall_tex = make_wall_texture(engine);
        let floor_tex = make_floor_texture(engine);
        let player_tex = make_player_texture(engine);

        let mut map = TileMap::new(MAP_W, MAP_H, TILE_SIZE);
        let wall_id = map.add_tile(TileDef::solid(wall_tex));

        let mut floor_map = TileMap::new(MAP_W, MAP_H, TILE_SIZE);
        let floor_id = floor_map.add_tile(TileDef::solid(floor_tex));

        for row in 0..MAP_H {
            for col in 0..MAP_W {
                floor_map.set(col, row, Some(floor_id));
                if LEVEL[row * MAP_W + col] == 1 {
                    map.set(col, row, Some(wall_id));
                }
            }
        }

        let player_pos = Vec2::new(
            2.0 * TILE_SIZE + TILE_SIZE * 0.5,
            2.0 * TILE_SIZE + TILE_SIZE * 0.5,
        );

        Self {
            map,
            floor_map,
            player_tex,
            player_pos,
        }
    }

    fn update(&mut self, engine: &Engine) {
        let dt = engine.dt();
        let input = engine.input();

        let mut dir = Vec2::ZERO;
        if input.is_key_down(KeyCode::KeyW) || input.is_key_down(KeyCode::ArrowUp) {
            dir.y += 1.0;
        }
        if input.is_key_down(KeyCode::KeyS) || input.is_key_down(KeyCode::ArrowDown) {
            dir.y -= 1.0;
        }
        if input.is_key_down(KeyCode::KeyA) || input.is_key_down(KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if input.is_key_down(KeyCode::KeyD) || input.is_key_down(KeyCode::ArrowRight) {
            dir.x += 1.0;
        }

        if dir != Vec2::ZERO {
            dir = dir.normalize();
        }

        self.player_pos += dir * PLAYER_SPEED * dt;

        let half = PLAYER_SIZE * 0.5;
        let player_rect = Rect::new(
            self.player_pos.x - half,
            self.player_pos.y - half,
            PLAYER_SIZE,
            PLAYER_SIZE,
        );
        if let Some(mtv) = self.map.collide_rect(&player_rect) {
            self.player_pos += mtv;
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.camera.position = self.player_pos;

        self.floor_map.draw(frame);
        self.map.draw(frame);

        let half = PLAYER_SIZE * 0.5;
        frame.draw(
            self.player_tex,
            Vec2::new(self.player_pos.x - half, self.player_pos.y - half),
            Vec2::new(PLAYER_SIZE, PLAYER_SIZE),
        );

        let screen = engine.window_size();
        let atlas = engine.font_atlas();
        let hud = frame.canvas(0);
        hud.rect(0.0, 0.0, screen.0 as f32, 36.0, Color::new(0.08, 0.07, 0.1, 0.85), screen);
        hud.text(12.0, 8.0, "TileMap Demo - WASD/Arrows to move", 16.0, Color::WHITE, screen, atlas);

        let pos_text = format!("pos: ({:.0}, {:.0})", self.player_pos.x, self.player_pos.y);
        let col = (self.player_pos.x / TILE_SIZE) as usize;
        let row = (self.player_pos.y / TILE_SIZE) as usize;
        let cell_text = format!("cell: ({}, {})", col, row);
        hud.text(12.0, screen.1 as f32 - 40.0, &pos_text, 13.0, Color::YELLOW, screen, atlas);
        hud.text(12.0, screen.1 as f32 - 22.0, &cell_text, 13.0, Color::new(0.6, 0.8, 1.0, 1.0), screen, atlas);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<TileMapDemo>(EngineConfig {
        title: "Feature: TileMap".into(),
        width: 640,
        height: 480,
        ..Default::default()
    })
}

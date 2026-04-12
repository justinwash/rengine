use glam::Vec2;

pub fn iso_to_screen(col: i32, row: i32, tile_width: f32, tile_height: f32) -> Vec2 {
    let x = (col - row) as f32 * (tile_width / 2.0);
    let y = (col + row) as f32 * (tile_height / 2.0);
    Vec2::new(x, -y)
}

pub fn screen_to_iso(screen: Vec2, tile_width: f32, tile_height: f32) -> (i32, i32) {
    let sx = screen.x;
    let sy = -screen.y;
    let col = (sx / (tile_width / 2.0) + sy / (tile_height / 2.0)) / 2.0;
    let row = (sy / (tile_height / 2.0) - sx / (tile_width / 2.0)) / 2.0;
    (col.round() as i32, row.round() as i32)
}

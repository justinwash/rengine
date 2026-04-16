use rengine::*;

struct NineSliceDemo {
    panel: NineSlice,
    quit: bool,
    time: f32,
}

fn make_panel_texture(engine: &mut Engine) -> TextureId {
    let size = 32u32;
    let border = 8u32;
    let mut canvas = pixelart::PixelCanvas::new(size, size);

    let corner_color = Color::from_rgba8(40, 45, 65, 255);
    canvas.fill(corner_color);

    let edge_color = Color::from_rgba8(55, 60, 85, 255);
    canvas.fill_rect(
        border as i32,
        0,
        (size - border * 2) as i32,
        border as i32,
        edge_color,
    );
    canvas.fill_rect(
        border as i32,
        (size - border) as i32,
        (size - border * 2) as i32,
        border as i32,
        edge_color,
    );
    canvas.fill_rect(
        0,
        border as i32,
        border as i32,
        (size - border * 2) as i32,
        edge_color,
    );
    canvas.fill_rect(
        (size - border) as i32,
        border as i32,
        border as i32,
        (size - border * 2) as i32,
        edge_color,
    );

    let center_color = Color::from_rgba8(70, 78, 110, 255);
    canvas.fill_rect(
        border as i32,
        border as i32,
        (size - border * 2) as i32,
        (size - border * 2) as i32,
        center_color,
    );

    let outline = Color::from_rgba8(120, 130, 180, 255);
    for i in 0..size as i32 {
        canvas.set(i, 0, outline);
        canvas.set(i, (size - 1) as i32, outline);
        canvas.set(0, i, outline);
        canvas.set((size - 1) as i32, i, outline);
    }

    let highlight = Color::from_rgba8(90, 100, 145, 255);
    for i in 1..(size - 1) as i32 {
        canvas.set(i, 1, highlight);
        canvas.set(1, i, highlight);
    }

    engine.create_texture(size, size, &canvas.into_bytes())
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Game for NineSliceDemo {
    fn new(engine: &mut Engine) -> Self {
        let tex = make_panel_texture(engine);
        let panel = NineSlice::uniform(tex, 32, 32, 8);

        Self {
            panel,
            quit: false,
            time: 0.0,
        }
    }

    fn update(&mut self, engine: &Engine, _frame: &mut Frame) {
        if engine.input().is_key_pressed(KeyCode::Escape) {
            self.quit = true;
        }
        self.time += engine.time().dt();
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        let (sw, sh) = engine.window_size();
        let atlas = engine.font_atlas();
        let t = self.time;
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;

        let world = |sx: f32, sy: f32, h: f32| -> Vec2 { Vec2::new(sx - hw, hh - sy - h) };

        let label_color = Color::from_rgba8(200, 200, 255, 255);
        let anim_label_color = Color::from_rgba8(150, 255, 220, 255);
        let ls = 11.0;

        let r1_lbl_y = 66.0;
        let r1_top = 82.0;
        let panels1: &[(&str, f32, f32, f32)] = &[
            ("Small (60x40)", 20.0, 60.0, 40.0),
            ("Medium (150x100)", 120.0, 150.0, 100.0),
            ("Wide (300x40)", 310.0, 300.0, 40.0),
        ];
        for &(_, sx, w, h) in panels1 {
            frame.draw_nine_slice(&self.panel, world(sx, r1_top, h), Vec2::new(w, h));
        }
        frame.draw(
            self.panel.texture,
            world(660.0, r1_top, 32.0),
            Vec2::new(32.0, 32.0),
        );
        frame.draw(
            self.panel.texture,
            world(740.0, r1_top, 50.0),
            Vec2::new(100.0, 50.0),
        );

        let r2_lbl_y = 210.0;
        let r2_top = 226.0;
        let panels2: &[(&str, f32, f32, f32)] = &[
            ("Tall (60x160)", 20.0, 60.0, 160.0),
            ("Large (300x140)", 120.0, 300.0, 140.0),
        ];
        for &(_, sx, w, h) in panels2 {
            frame.draw_nine_slice(&self.panel, world(sx, r2_top, h), Vec2::new(w, h));
        }
        let tinted = self
            .panel
            .clone()
            .with_color(Color::from_rgba8(255, 180, 100, 255));
        frame.draw_nine_slice(
            &tinted,
            world(480.0, r2_top, 130.0),
            Vec2::new(160.0, 130.0),
        );

        let r3_lbl_y = 420.0;
        let r3_top = 436.0;
        let anim_w1 = lerp(50.0, 150.0, (t * 0.8).sin() * 0.5 + 0.5);
        let anim_h1 = lerp(40.0, 120.0, (t * 1.2).sin() * 0.5 + 0.5);
        let anim_w2 = lerp(80.0, 280.0, (t * 0.6).sin() * 0.5 + 0.5);
        let breath = lerp(50.0, 120.0, (t * 1.5).sin() * 0.5 + 0.5);

        let anim_panel = self
            .panel
            .clone()
            .with_color(Color::from_rgba8(100, 220, 180, 200));
        frame.draw_nine_slice(
            &anim_panel,
            world(20.0, r3_top, anim_h1),
            Vec2::new(anim_w1, anim_h1),
        );
        frame.draw_nine_slice(
            &anim_panel,
            world(220.0, r3_top, 60.0),
            Vec2::new(anim_w2, 60.0),
        );
        frame.draw_nine_slice(
            &anim_panel,
            world(600.0, r3_top, breath),
            Vec2::new(breath, breath),
        );

        frame.clear_color = Color::from_rgba8(15, 15, 25, 255);
        let canvas = frame.canvas(0);

        let p = |sx: f32, sy: f32, _size: f32| -> (f32, f32) { (sx - hw, hh - sy) };

        let (tx, ty) = p(20.0, 10.0, 28.0);
        canvas.text(
            tx,
            ty,
            "NineSlice Feature Demo",
            28.0,
            Color::WHITE,
            atlas,
        );
        let (tx, ty) = p(20.0, 42.0, 14.0);
        canvas.text(
            tx,
            ty,
            "Same 32x32 texture drawn at different sizes - corners stay sharp",
            14.0,
            Color::from_rgba8(180, 180, 180, 255),
            atlas,
        );

        for &(lbl, sx, _, _) in panels1 {
            let (tx, ty) = p(sx, r1_lbl_y, ls);
            canvas.text(tx, ty, lbl, ls, label_color, atlas);
        }
        let (tx, ty) = p(660.0, r1_lbl_y, ls);
        canvas.text(tx, ty, "Source (1:1)", ls, label_color, atlas);
        let (tx, ty) = p(740.0, r1_lbl_y, ls);
        canvas.text(tx, ty, "Naive stretch", ls, label_color, atlas);

        for &(lbl, sx, _, _) in panels2 {
            let (tx, ty) = p(sx, r2_lbl_y, ls);
            canvas.text(tx, ty, lbl, ls, label_color, atlas);
        }
        let (tx, ty) = p(480.0, r2_lbl_y, ls);
        canvas.text(
            tx,
            ty,
            "Tinted (160x130)",
            ls,
            Color::from_rgba8(255, 200, 150, 255),
            atlas,
        );

        let (tx, ty) = p(20.0, r3_lbl_y, ls);
        canvas.text(
            tx,
            ty,
            "Animated (resizing)",
            ls,
            anim_label_color,
            atlas,
        );
        let (tx, ty) = p(220.0, r3_lbl_y, ls);
        canvas.text(
            tx,
            ty,
            "Animated (width)",
            ls,
            anim_label_color,
            atlas,
        );
        let (tx, ty) = p(600.0, r3_lbl_y, ls);
        canvas.text(
            tx,
            ty,
            "Animated (breathing)",
            ls,
            anim_label_color,
            atlas,
        );

        let (tx, ty) = p(20.0, sh as f32 - 24.0, 14.0);
        canvas.text(
            tx,
            ty,
            "ESC to quit",
            14.0,
            Color::from_rgba8(120, 120, 120, 255),
            atlas,
        );
    }

    fn should_exit(&self) -> bool {
        self.quit
    }
}

fn main() {
    let config = EngineConfig {
        title: "Feature: Nine-Slice".into(),
        width: 960,
        height: 700,
        show_fps: false,
        ..Default::default()
    };
    run::<NineSliceDemo>(config).unwrap();
}

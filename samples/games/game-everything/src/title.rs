use crate::game::GameScene;
use crate::state::*;
use rengine::*;

pub struct TitleScene {
    blink_timer: f32,
    panel: Option<NineSlice>,
}

impl TitleScene {
    pub fn new() -> Self {
        Self {
            blink_timer: 0.0,
            panel: None,
        }
    }
}

impl Scene for TitleScene {
    fn on_enter(&mut self, engine: &mut Engine, globals: &mut Globals) {
        println!("[TitleScene] on_enter");
        if let Some(counter) = globals.get_mut::<TransitionCounter>() {
            counter.0 += 1;
        }

        let sz = 16u32;
        let bd = 3u32;
        let mut pc = pixelart::PixelCanvas::new(sz, sz);
        pc.fill(Color::new(0.12, 0.08, 0.25, 0.85));
        let edge = Color::new(0.5, 0.35, 0.9, 1.0);
        for i in 0..sz as i32 {
            pc.set(i, 0, edge);
            pc.set(i, (sz - 1) as i32, edge);
            pc.set(0, i, edge);
            pc.set((sz - 1) as i32, i, edge);
        }
        let tex = engine.create_texture(sz, sz, &pc.into_bytes());
        self.panel = Some(NineSlice::uniform(tex, sz, sz, bd).with_z_order(-1));
        println!("[FEATURE OK] NineSlice::uniform + with_z_order — title card panel");

        if let Some(demo) = globals.get_mut::<DemoConfig>() {
            demo.log_feature("Scene::on_enter");
            demo.log_feature("Globals::get_mut");
            demo.log_feature("NineSlice::uniform");
            demo.log_feature("NineSlice::with_z_order");
            demo.log_feature("frame.draw_nine_slice");
        }
    }

    fn update(&mut self, engine: &Engine, globals: &mut Globals, frame: &mut Frame) -> SceneOp {
        self.blink_timer += engine.dt();

        if let Some(demo) = globals.get_mut::<DemoConfig>() {
            if demo.enabled {
                demo.frame += 1;
                demo.log_feature("TimeState::dt");
                if demo.frame > 5 {
                    println!("[TitleScene] demo: auto-switching to GameScene");
                    demo.log_feature("SceneOp::Switch (Title->Game)");
                    return SceneOp::Switch(Box::new(GameScene::default()));
                }
            }
        }

        if engine.action_pressed("confirm") {
            return SceneOp::Switch(Box::new(GameScene::default()));
        }
        if engine.action_pressed("quit") {
            return SceneOp::Quit;
        }

        frame.clear_color = Color::new(0.1, 0.05, 0.2, 1.0);

        let (sw, sh) = engine.window_size();
        let hw = sw as f32 / 2.0;
        let hh = sh as f32 / 2.0;

        if let Some(panel) = &self.panel {
            frame.draw_nine_slice(panel, Vec2::new(-320.0, 40.0), Vec2::new(640.0, 280.0));
            let tinted = panel.clone().with_color(Color::new(0.3, 1.0, 0.5, 0.8));
            frame.draw_nine_slice(
                &tinted,
                Vec2::new(-hw + 5.0, -hh + 5.0),
                Vec2::new(260.0, 40.0),
            );
        }

        let canvas = frame.canvas(0);

        canvas.text(
            -hw + 200.0,
            hh - 100.0,
            "RENGINE KITCHEN SINK",
            32.0,
            Color::YELLOW,
        );

        if (self.blink_timer * 2.0).sin() > 0.0 {
            canvas.text(
                -hw + 220.0,
                hh - 250.0,
                "Press ENTER to start",
                18.0,
                Color::WHITE,
            );
        }

        let transitions = globals.get::<TransitionCounter>().map_or(0, |c| c.0);
        canvas.text(
            -hw + 10.0,
            -hh + 50.0,
            &format!("Scene transitions: {}", transitions),
            12.0,
            Color::GREEN,
        );

        if engine.gamepads_connected() > 0 {
            canvas.text(
                -hw + 220.0,
                hh - 300.0,
                "(Gamepad detected: press A)",
                14.0,
                Color::ORANGE,
            );
        }

        SceneOp::Continue
    }

    fn on_exit(&mut self, _engine: &Engine, _globals: &Globals) {
        println!("[TitleScene] on_exit");
    }
}

mod gameplay;
mod scenes;
mod state;

use std::path::PathBuf;

use rengine::*;
use scenes::MenuScene;
use state::{sync_gamepad_pairing, SessionState};

fn main() {
    rengine::run_with_scenes(
        EngineConfig {
            title: "St. Louis Game Jam".into(),
            width: 1920,
            height: 1080,
            fixed_dt: 1.0 / 60.0,
            render_width: Some(640),
            render_height: Some(360),
            scale_mode: ScaleMode::Letterbox,
            show_fps: false,
            ..Default::default()
        },
        |engine, globals| {
            engine.set_asset_root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));

            globals.set(SessionState::default());
            sync_gamepad_pairing(engine);

            Box::new(MenuScene::new()) as Box<dyn Scene>
        },
    )
    .unwrap();
}

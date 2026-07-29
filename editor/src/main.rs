mod app;
mod scene;

use app::RengineNativeEditor;
use rengine::{run, EngineConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --headless + RENGINE_PLAY_SCRIPT drive the editor the same way the game
    // samples do, so editor UI changes get a scripted click+screenshot check
    // instead of only a manual one.
    let headless = std::env::args().any(|a| a == "--headless");
    run::<RengineNativeEditor>(EngineConfig {
        title: "Rengine Editor".into(),
        width: 1440,
        height: 900,
        show_fps: false,
        headless,
        ..Default::default()
    })
}

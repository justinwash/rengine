mod app;
mod scene;

use app::RengineNativeEditor;
use rengine::{run, EngineConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run::<RengineNativeEditor>(EngineConfig {
        title: "Rengine Editor".into(),
        width: 1440,
        height: 900,
        show_fps: false,
        ..Default::default()
    })
}

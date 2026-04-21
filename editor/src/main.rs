mod app;
mod scene;

use app::RengineEditorApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("Rengine Editor"),
        ..Default::default()
    };

    eframe::run_native(
        "Rengine Editor",
        native_options,
        Box::new(|cc| Ok(Box::new(RengineEditorApp::new(cc)))),
    )
}

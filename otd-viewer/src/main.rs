//! OTD Viewer - GUI viewer for OTD glass cutting layouts.

mod app;
mod canvas;
mod layers;
mod theme;
mod transform;

use app::ViewerApp;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Check for command-line file argument
    let initial_file: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("OTD Viewer"),
        // Don't block when window is not visible (prevents "not responding" on focus loss)
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "OTD Viewer",
        options,
        Box::new(move |cc| Ok(Box::new(ViewerApp::new(cc, initial_file)))),
    )
}

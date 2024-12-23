use eframe::egui;
use env_logger;

mod file_selector;
mod magick_tiler;
mod radio_button_group;

use magick_tiler::MagickTilerApp;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Initialize logger

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        min_window_size: Some(egui::vec2(400.0, 300.0)),
        ..Default::default()
    };

    eframe::run_native(
        "MagickTiler",
        options,
        Box::new(|cc| Box::new(MagickTilerApp::new(cc))),
    )
}

// Hide the console window on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod chrome;
mod clipboard;
mod coords;
mod theme;

fn main() -> eframe::Result {
    let viewport = egui::ViewportBuilder::default()
        .with_title("WARDOGS // Artillery Range")
        .with_app_id("wardogs-range-calc")
        .with_inner_size([560.0, 760.0])
        .with_min_inner_size([480.0, 640.0]);
    let options = eframe::NativeOptions {
        viewport,
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native("WARDOGS // Artillery Range", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}

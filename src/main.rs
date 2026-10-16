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
        .with_inner_size([app::WINDOW_W, app::WINDOW_H])
        .with_min_inner_size([app::WINDOW_W, app::WINDOW_H])
        .with_max_inner_size([app::WINDOW_W, app::WINDOW_H])
        .with_resizable(false)
        .with_maximize_button(false);
    let options = eframe::NativeOptions {
        viewport,
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native("WARDOGS // Artillery Range", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}

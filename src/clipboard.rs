//! Watches the clipboard and reports every newly copied game coordinate.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use crate::coords::{self, Coord};

const POLL_EVERY: Duration = Duration::from_millis(150);

/// A background thread polling the clipboard. Text that was already there when it started, or
/// that is copied while paused, is never reported.
pub struct Watcher {
    pub coords: Receiver<Coord>,
    paused: Arc<AtomicBool>,
    /// False when the system clipboard can't be opened at all.
    pub available: bool,
}

impl Watcher {
    pub fn start(ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let paused = Arc::new(AtomicBool::new(false));
        let available = arboard::Clipboard::new().is_ok();
        if available {
            let paused = paused.clone();
            thread::Builder::new()
                .name("clipboard-watcher".into())
                .spawn(move || {
                    let Ok(mut clipboard) = arboard::Clipboard::new() else { return };
                    let mut last = clipboard.get_text().ok();
                    loop {
                        thread::sleep(POLL_EVERY);
                        // The clipboard is briefly locked while another app writes to it: try
                        // again on the next tick.
                        let Ok(text) = clipboard.get_text() else { continue };
                        if last.as_deref() == Some(text.as_str()) {
                            continue;
                        }
                        last = Some(text);
                        if paused.load(Ordering::Relaxed) {
                            continue;
                        }
                        if let Some(coord) = last.as_deref().and_then(|t| coords::parse(t, false)) {
                            if tx.send(coord).is_err() {
                                return;
                            }
                            ctx.request_repaint();
                        }
                    }
                })
                .ok();
        }
        Self { coords: rx, paused, available }
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }
}

//! Watches the clipboard via OS events and reports every newly copied game coordinate.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;

use clipboard_rs::{
    Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher,
    ClipboardWatcherContext,
};

use crate::coords::{self, Coord};

struct CoordHandler {
    ctx: ClipboardContext,
    tx: mpsc::Sender<Coord>,
    egui_ctx: egui::Context,
    paused: Arc<AtomicBool>,
}

impl ClipboardHandler for CoordHandler {
    fn on_clipboard_change(&mut self) {
        if self.paused.load(Ordering::Relaxed) {
            return;
        }

        if let Ok(text) = self.ctx.get_text()
            && let Some(coord) = coords::parse(&text, false)
            && self.tx.send(coord).is_ok()
        {
            self.egui_ctx.request_repaint();
        }
    }
}

pub struct Watcher {
    pub coords: Receiver<Coord>,
    paused: Arc<AtomicBool>,
    shutdown: Option<clipboard_rs::WatcherShutdown>,
    pub available: bool,
}

impl Watcher {
    pub fn start(ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let paused = Arc::new(AtomicBool::new(false));

        let (available, shutdown) = match (ClipboardContext::new(), ClipboardWatcherContext::new()) {
            (Ok(clip_ctx), Ok(mut watcher)) => {
                let handler = CoordHandler {
                    ctx: clip_ctx,
                    tx,
                    egui_ctx: ctx,
                    paused: paused.clone(),
                };

                let shutdown_handle = watcher.add_handler(handler).get_shutdown_channel();

                // Run watcher on a dedicated OS event thread
                thread::Builder::new()
                    .name("clipboard-watcher".into())
                    .spawn(move || {
                        watcher.start_watch();
                    })
                    .ok();

                (true, Some(shutdown_handle))
            }
            _ => (false, None),
        };

        Self {
            coords: rx,
            paused,
            shutdown,
            available,
        }
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            shutdown.stop();
        }
    }
}

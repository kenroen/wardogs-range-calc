//! The calculator window.

use egui::{Align, Color32, CornerRadius, FontFamily, FontId, Layout, Margin, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, vec2};
use serde::{Deserialize, Serialize};

use crate::chrome::{self, WindowButton};
use crate::clipboard::Watcher;
use crate::coords::{self, Coord, Reach, Solution, WEAPONS};
use crate::theme;

/// Fixed window dimensions.
pub const WINDOW_W: f32 = 560.0;
pub const WINDOW_H: f32 = 880.0;

/// How long a field glows after the clipboard fills it, in seconds.
const FLASH_SECS: f64 = 0.9;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedTarget {
    pub name: String,
    pub coords: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Slot {
    Me,
    Target,
}

impl Slot {
    fn name(self) -> &'static str {
        match self {
            Slot::Me => "ME",
            Slot::Target => "TARGET",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Slot::Me => theme::ME,
            Slot::Target => theme::TARGET,
        }
    }
}

/// Where copied coordinates go.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Route {
    /// Into ME while it is empty, otherwise into TARGET.
    Auto,
    Me,
    Target,
}

/// What survives a restart.
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    me: String,
    target: String,
    route: Route,
    listening: bool,
    metres_per_unit: f64,
    y_north: bool,
    on_top: bool,
    saved_targets: Vec<SavedTarget>,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            me: String::new(),
            target: String::new(),
            route: Route::Auto,
            listening: true,
            metres_per_unit: 100.0,
            y_north: true,
            on_top: false,
            saved_targets: Vec::new(),
        }
    }
}

pub struct App {
    s: Saved,
    watcher: Watcher,
    /// When each field was last filled from the clipboard (`egui` time).
    flashed_at: [Option<f64>; 2],
    last_clip: Option<(Coord, Slot)>,
    applied_on_top: Option<bool>,
    target_name: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let s: Saved = cc.storage.and_then(|st| eframe::get_value(st, eframe::APP_KEY)).unwrap_or_default();
        let watcher = Watcher::start(cc.egui_ctx.clone());
        watcher.set_paused(!s.listening);
        Self { s, watcher, flashed_at: [None; 2], last_clip: None, applied_on_top: None, target_name: String::new() }
    }

    fn text_mut(&mut self, slot: Slot) -> &mut String {
        match slot {
            Slot::Me => &mut self.s.me,
            Slot::Target => &mut self.s.target,
        }
    }

    fn coord(&self, slot: Slot) -> Option<Coord> {
        let text = match slot {
            Slot::Me => &self.s.me,
            Slot::Target => &self.s.target,
        };
        coords::parse(text, true)
    }

    /// The field the next copied coordinate will go into.
    fn next_slot(&self) -> Slot {
        match self.s.route {
            Route::Me => Slot::Me,
            Route::Target => Slot::Target,
            Route::Auto if self.s.me.trim().is_empty() => Slot::Me,
            Route::Auto => Slot::Target,
        }
    }

    fn receive(&mut self, coord: Coord, now: f64) {
        let slot = self.next_slot();
        *self.text_mut(slot) = coord.to_string();
        self.flashed_at[slot as usize] = Some(now);
        self.last_clip = Some((coord, slot));
    }

    fn set_listening(&mut self, on: bool) {
        self.s.listening = on;
        self.watcher.set_paused(!on);
    }

    fn title_bar(&mut self, ui: &mut Ui) {
        chrome::title_bar_drag(ui);
        ui.horizontal_centered(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
            crosshair(ui, rect.center(), 9.0, theme::ACCENT);
            ui.add(
                egui::Label::new(RichText::new("WARDOGS").font(FontId::new(18.0, FontFamily::Monospace)).color(theme::ACCENT).strong())
                    .selectable(false),
            );
            ui.add(
                egui::Label::new(RichText::new("// ARTILLERY RANGE").font(FontId::new(15.0, FontFamily::Monospace)).color(theme::MUTED))
                    .selectable(false),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let h = chrome::TITLE_H;
                if chrome::window_button(ui, WindowButton::Close, h).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if chrome::window_button(ui, WindowButton::Minimize, h).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            });
        });
    }

    fn routing(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            caption(ui, "CLIPBOARD →");
            for (route, label, color) in [
                (Route::Auto, "AUTO", theme::ACCENT),
                (Route::Me, "ME", theme::ME),
                (Route::Target, "TARGET", theme::TARGET),
            ] {
                let tip = match route {
                    Route::Auto => "Copied coordinates fill ME while it is empty, then TARGET",
                    Route::Me => "Copied coordinates always go into ME",
                    Route::Target => "Copied coordinates always go into TARGET",
                };
                if hud_button(ui, label, color, self.s.route == route).on_hover_text(tip).clicked() {
                    self.s.route = route;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let on = self.s.listening && self.watcher.available;
                let label = if on { "● LISTENING" } else { "○ PAUSED" };
                let color = if on { theme::GOOD } else { theme::MUTED };
                let tip = if !self.watcher.available {
                    "The clipboard can't be read on this system"
                } else if on {
                    "Click to stop watching the clipboard"
                } else {
                    "Click to watch the clipboard for coordinates"
                };
                if hud_button(ui, label, color, false).on_hover_text(tip).clicked() && self.watcher.available {
                    self.set_listening(!self.s.listening);
                }
            });
        });
    }

    fn coord_card(&mut self, ui: &mut Ui, slot: Slot, title: &str) {
        let now = ui.input(|i| i.time);
        let color = slot.color();
        let glow = self.flashed_at[slot as usize].map_or(0.0, |t| (1.0 - (now - t) / FLASH_SECS).clamp(0.0, 1.0) as f32);
        if glow > 0.0 {
            ui.ctx().request_repaint();
        }
        let receiving = self.s.listening && self.watcher.available && self.next_slot() == slot;
        let coord = self.coord(slot);
        let border = theme::LINE.lerp_to_gamma(color, 0.25 + 0.75 * glow);
        let frame = egui::Frame::new()
            .fill(theme::PANEL.lerp_to_gamma(color, 0.12 * glow))
            .stroke(Stroke::new(1.0, border))
            .inner_margin(Margin { left: 16, right: 12, top: 10, bottom: 10 });
        let resp = frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("■ {}", slot.name())).font(FontId::new(17.0, FontFamily::Monospace)).color(color).strong());
                caption(ui, title);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if hud_button(ui, "CLEAR", theme::MUTED, false).on_hover_text(format!("Clear {}", slot.name())).clicked() {
                        self.text_mut(slot).clear();
                        if slot == Slot::Target {
                            self.target_name.clear();
                        }
                        self.flashed_at[slot as usize] = None;
                    }
                    if receiving {
                        ui.label(RichText::new("◀ NEXT PASTE").font(FontId::new(13.0, FontFamily::Monospace)).color(theme::ACCENT));
                    }
                });
            });
            let text = self.text_mut(slot);
            egui::TextEdit::singleline(text)
                .font(FontId::new(22.0, FontFamily::Monospace))
                .hint_text("x95.62, y104.98")
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(10, 8))
                .show(ui);
            let empty = text.trim().is_empty();
            ui.horizontal(|ui| match coord {
                Some(c) => {
                    small(ui, &format!("E {:.2}", c.x), theme::INK);
                    small(ui, &format!("N {:.2}", c.y), theme::INK);
                    small(ui, "✔ LOCKED", theme::GOOD);
                }
                None if empty => small(ui, "AWAITING COORDINATES", theme::MUTED),
                None => small(ui, "✖ UNREADABLE — USE x95.62, y104.98", theme::BAD),
            });
            if slot == Slot::Target {
                ui.add_space(4.0);
                ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, theme::LINE));
                ui.add_space(6.0);
                self.saved_targets_row(ui, coord, now);
            }
        });
        // A colour bar down the card's left edge.
        let r = resp.response.rect;
        let bar = Rect::from_min_size(r.min, vec2(4.0, r.height()));
        ui.painter().rect_filled(bar, CornerRadius::ZERO, color.gamma_multiply(0.6 + 0.4 * glow));
    }

    fn saved_targets_row(&mut self, ui: &mut Ui, current_coord: Option<Coord>, now: f64) {
        let formatted_current = current_coord.map(|c| c.to_string());
        let active_idx = formatted_current
            .as_deref()
            .and_then(|cur| self.s.saved_targets.iter().position(|t| t.coords == cur));

        // Row 1: Saved targets dropdown + delete button for the selected saved target.
        ui.horizontal(|ui| {
            caption(ui, "SAVED");
            let selected_label = match active_idx {
                Some(i) => format!("{} ({})", self.s.saved_targets[i].name, self.s.saved_targets[i].coords),
                None if self.s.saved_targets.is_empty() => "No saved targets".to_owned(),
                None => "Select target…".to_owned(),
            };
            let mut chosen: Option<usize> = None;
            egui::ComboBox::from_id_salt("saved-targets-combo")
                .width((ui.available_width() - if active_idx.is_some() { 68.0 } else { 0.0 }).max(140.0))
                .selected_text(RichText::new(selected_label).font(FontId::new(14.0, FontFamily::Monospace)).color(theme::INK))
                .show_ui(ui, |ui| {
                    for (i, entry) in self.s.saved_targets.iter().enumerate() {
                        let is_sel = active_idx == Some(i);
                        let label = format!("{}  —  {}", entry.name, entry.coords);
                        if ui
                            .selectable_label(is_sel, RichText::new(label).font(FontId::new(14.0, FontFamily::Monospace)))
                            .clicked()
                        {
                            chosen = Some(i);
                        }
                    }
                });
            if let Some(i) = chosen {
                self.s.target = self.s.saved_targets[i].coords.clone();
                self.target_name = self.s.saved_targets[i].name.clone();
                self.flashed_at[Slot::Target as usize] = Some(now);
            }
            if let Some(i) = active_idx
                && hud_button(ui, "DEL", theme::BAD, false)
                    .on_hover_text("Delete this saved target")
                    .clicked()
            {
                self.s.saved_targets.remove(i);
                self.target_name.clear();
            }
        });

        // Row 2: Name input + SAVE button to save the current target coordinate.
        ui.horizontal(|ui| {
            let can_save = current_coord.is_some() && !self.target_name.trim().is_empty();
            let save_w = 72.0;
            let edit_resp = ui.add(
                egui::TextEdit::singleline(&mut self.target_name)
                    .font(FontId::new(15.0, FontFamily::Monospace))
                    .hint_text("Target name (e.g. North Bunker)")
                    .desired_width((ui.available_width() - save_w).max(120.0))
                    .margin(Margin::symmetric(8, 5)),
            );
            let enter_pressed = edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let save_color = if can_save { theme::ACCENT } else { theme::MUTED };
            let save_clicked = hud_button(ui, "SAVE", save_color, false)
                .on_hover_text("Save current target coordinates with this name")
                .clicked();
            if (save_clicked || enter_pressed)
                && let Some(coord) = current_coord
            {
                let name = self.target_name.trim().to_owned();
                if !name.is_empty() {
                    let coords = coord.to_string();
                    if let Some(existing) = self.s.saved_targets.iter_mut().find(|t| t.name.eq_ignore_ascii_case(&name)) {
                        existing.name = name;
                        existing.coords = coords.clone();
                    } else {
                        self.s.saved_targets.push(SavedTarget { name, coords: coords.clone() });
                    }
                    self.s.target = coords;
                }
            }
        });
    }

    fn solution(&mut self, ui: &mut Ui) {
        let solution = self.coord(Slot::Me).zip(self.coord(Slot::Target)).map(|(me, target)| Solution::between(me, target, self.s.metres_per_unit, self.s.y_north));
        egui::Frame::new().fill(theme::FIELD).stroke(Stroke::new(1.0, theme::LINE_STRONG)).inner_margin(Margin::same(14)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let Some(s) = solution else {
                caption(ui, "FIRING SOLUTION");
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("— — —").font(FontId::new(46.0, FontFamily::Monospace)).color(theme::LINE_STRONG));
                    small(ui, "ENTER OR COPY BOTH POSITIONS", theme::MUTED);
                });
                ui.add_space(10.0);
                return;
            };
            ui.horizontal(|ui| {
                caption(ui, "RANGE");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if hud_button(ui, "COPY", theme::MUTED, false).on_hover_text("Copy the range and bearing").clicked() {
                        ui.ctx().copy_text(format!("{:.0} m, {:.1}° ({:.0} mil)", s.range_m, s.bearing_deg, s.bearing_mils()));
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{:.0}", s.range_m)).font(FontId::new(56.0, FontFamily::Monospace)).color(theme::ACCENT).strong());
                ui.label(RichText::new("m").font(FontId::new(26.0, FontFamily::Monospace)).color(theme::MUTED));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(54.0, 54.0), Sense::hover());
                    compass_rose(ui, rect, s.bearing_deg);
                    ui.allocate_ui_with_layout(vec2(160.0, 54.0), Layout::top_down(Align::Max), |ui| {
                        ui.label(RichText::new(format!("{:05.1}°  {}", s.bearing_deg, s.compass())).font(FontId::new(23.0, FontFamily::Monospace)).color(theme::INK));
                        small(ui, &format!("{:.0} MIL", s.bearing_mils()), theme::MUTED);
                    });
                });
            });
            ui.horizontal(|ui| {
                small(ui, &format!("ΔE {:+.0} m", s.east_m), theme::MUTED);
                small(ui, &format!("ΔN {:+.0} m", s.north_m), theme::MUTED);
            });
            ui.add_space(6.0);
            ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, theme::LINE));
            ui.add_space(8.0);
            for w in WEAPONS {
                ui.horizontal(|ui| {
                    small(ui, w.name, theme::INK);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (text, color) = match w.reach(s.range_m) {
                            Reach::TooClose => (format!("TOO CLOSE · MIN {:.0} m", w.min_m), theme::BAD),
                            Reach::TooFar => (format!("OUT OF RANGE · MAX {:.0} m", w.max_m), theme::BAD),
                            Reach::HighArc => ("IN RANGE · HIGH ARC".to_owned(), theme::GOOD),
                            Reach::HighOrLowArc => ("IN RANGE · HIGH / LOW ARC".to_owned(), theme::GOOD),
                        };
                        small(ui, &text, color);
                    });
                });
            }
        });
    }

    fn actions(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if hud_button(ui, "CLEAR ALL", theme::ACCENT, false).on_hover_text("Clear both positions").clicked() {
                self.s.me.clear();
                self.s.target.clear();
                self.target_name.clear();
                self.flashed_at = [None; 2];
                self.last_clip = None;
            }
            if hud_button(ui, "CLEAR TARGET", theme::TARGET, false).on_hover_text("Keep your position, pick a new target").clicked() {
                self.s.target.clear();
                self.target_name.clear();
                self.flashed_at[Slot::Target as usize] = None;
            }
            if hud_button(ui, "⇅ SWAP", theme::MUTED, false).on_hover_text("Swap ME and TARGET").clicked() {
                std::mem::swap(&mut self.s.me, &mut self.s.target);
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| match self.last_clip {
            Some((c, slot)) => {
                small(ui, "LAST COPY", theme::MUTED);
                small(ui, &c.to_string(), theme::INK);
                small(ui, &format!("→ {}", slot.name()), slot.color());
            }
            None => small(ui, "COPY A POSITION IN GAME TO FILL A FIELD", theme::MUTED),
        });
    }

    fn settings(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new(RichText::new("MAP SETTINGS").font(FontId::new(14.0, FontFamily::Monospace)).color(theme::MUTED))
            .id_salt("map-settings")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    small(ui, "SCALE", theme::MUTED);
                    ui.add(egui::DragValue::new(&mut self.s.metres_per_unit).range(0.01..=10_000.0).speed(1.0).suffix(" m / unit"));
                    if hud_button(ui, "RESET", theme::MUTED, false).on_hover_text("One coordinate unit is 100 m").clicked() {
                        self.s.metres_per_unit = 100.0;
                    }
                });
                ui.horizontal(|ui| {
                    small(ui, "Y AXIS", theme::MUTED);
                    if hud_button(ui, "NORTH ↑", theme::ACCENT, self.s.y_north).on_hover_text("Y grows northwards (WARDOGS default)").clicked() {
                        self.s.y_north = true;
                    }
                    if hud_button(ui, "SOUTH ↓", theme::ACCENT, !self.s.y_north).on_hover_text("Y grows southwards").clicked() {
                        self.s.y_north = false;
                    }
                });
            });
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = ctx.input(|i| i.time);
        while let Ok(coord) = self.watcher.coords.try_recv() {
            self.receive(coord, now);
        }
        if self.applied_on_top != Some(self.s.on_top) {
            let level = if self.s.on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
            self.applied_on_top = Some(self.s.on_top);
            // Ensure any previously persisted smaller window size is reset to the fixed height.
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(WINDOW_W, WINDOW_H)));
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let inset = chrome::FRAME_INSET as i8;
        egui::Panel::top("title-bar")
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(Margin { left: inset + 10, right: inset - 2, top: 4, bottom: 0 }))
            .exact_size(chrome::TITLE_H)
            .resizable(false)
            .show_separator_line(false)
            .show(ui, |ui| self.title_bar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(Margin { left: inset + 10, right: inset + 10, top: 6, bottom: 4 }))
            .show(ui, |ui| {
                self.routing(ui);
                ui.add_space(8.0);
                self.coord_card(ui, Slot::Me, "GUN POSITION");
                ui.add_space(8.0);
                self.coord_card(ui, Slot::Target, "IMPACT POINT");
                ui.add_space(10.0);
                self.solution(ui);
                ui.add_space(10.0);
                self.actions(ui);
                ui.add_space(6.0);
                self.settings(ui);
            });
        chrome::window_frame(&ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.s);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::BG.to_normalized_gamma_f32()
    }
}

/// A muted mono caption.
fn caption(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(FontId::new(14.0, FontFamily::Monospace)).color(theme::MUTED));
}

fn small(ui: &mut Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).font(FontId::new(14.0, FontFamily::Monospace)).color(color));
}

/// A flat bracketed button; filled with `color` while `selected`.
fn hud_button(ui: &mut Ui, text: &str, color: Color32, selected: bool) -> Response {
    let font = FontId::new(14.0, FontFamily::Monospace);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let size = galley.size() + vec2(20.0, 10.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hovered = resp.hovered();
    let (fill, ink, stroke) = if selected {
        (color, theme::BG, color)
    } else if hovered {
        (color.gamma_multiply(0.15), color, color)
    } else {
        (Color32::TRANSPARENT, color, theme::LINE_STRONG)
    };
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::ZERO, fill);
    p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, stroke), StrokeKind::Inside);
    // Bright corner nicks.
    let nick = Stroke::new(1.0, color);
    for (c, dx, dy) in [(rect.left_top(), 1.0, 1.0), (rect.right_top(), -1.0, 1.0), (rect.left_bottom(), 1.0, -1.0), (rect.right_bottom(), -1.0, -1.0)] {
        let c = c + vec2(0.5 * dx, 0.5 * dy);
        p.line_segment([c, c + vec2(4.0 * dx, 0.0)], nick);
        p.line_segment([c, c + vec2(0.0, 4.0 * dy)], nick);
    }
    p.galley_with_override_text_color(rect.center() - galley.size() / 2.0, galley, ink);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, text));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn crosshair(ui: &Ui, c: egui::Pos2, r: f32, color: Color32) {
    let p = ui.painter();
    let stroke = Stroke::new(1.5, color);
    p.circle_stroke(c, r * 0.6, stroke);
    p.line_segment([c - vec2(r, 0.0), c - vec2(r * 0.3, 0.0)], stroke);
    p.line_segment([c + vec2(r * 0.3, 0.0), c + vec2(r, 0.0)], stroke);
    p.line_segment([c - vec2(0.0, r), c - vec2(0.0, r * 0.3)], stroke);
    p.line_segment([c + vec2(0.0, r * 0.3), c + vec2(0.0, r)], stroke);
}

/// A small compass with a needle pointing along `bearing_deg`.
fn compass_rose(ui: &Ui, rect: Rect, bearing_deg: f64) {
    let p = ui.painter();
    let c = rect.center();
    let r = rect.width().min(rect.height()) / 2.0 - 2.0;
    p.circle_stroke(c, r, Stroke::new(1.0, theme::LINE_STRONG));
    for k in 0..8 {
        let a = (k as f32) * std::f32::consts::FRAC_PI_4;
        let dir = vec2(a.sin(), -a.cos());
        let inner = if k % 2 == 0 { r - 5.0 } else { r - 3.0 };
        p.line_segment([c + dir * inner, c + dir * r], Stroke::new(1.0, theme::LINE_STRONG));
    }
    p.text(c + vec2(0.0, -r + 8.0), egui::Align2::CENTER_CENTER, "N", FontId::new(10.0, FontFamily::Monospace), theme::MUTED);
    let a = (bearing_deg as f32).to_radians();
    let dir = vec2(a.sin(), -a.cos());
    p.line_segment([c - dir * (r * 0.35), c + dir * (r - 4.0)], Stroke::new(2.0, theme::ACCENT));
    p.circle_filled(c, 2.0, theme::ACCENT);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coord(x: f64, y: f64) -> Coord {
        Coord { x, y }
    }

    /// Verifies clipboard routing and renders the window with a solution showing.
    /// Set `WARDOGS_SNAPSHOT` to a PNG path to save the rendered frame.
    #[test]
    fn app_routing_and_render() {
        let mut harness = egui_kittest::Harness::builder().with_size(vec2(WINDOW_W, WINDOW_H)).build_eframe(|cc| App::new(cc));
        {
            let app = harness.state_mut();
            app.s = Saved::default();
            app.receive(coord(1.0, 2.0), 0.0);
            assert_eq!(app.s.me, "x1.00, y2.00");
            app.receive(coord(3.0, 4.0), 0.0);
            assert_eq!(app.s.target, "x3.00, y4.00");
            // Further copies keep retargeting while ME stays put.
            app.receive(coord(5.0, 6.0), 0.0);
            assert_eq!(app.s.me, "x1.00, y2.00");
            assert_eq!(app.s.target, "x5.00, y6.00");
            app.s.route = Route::Me;
            app.receive(coord(7.0, 8.0), 0.0);
            assert_eq!(app.s.me, "x7.00, y8.00");

            // Load sample game shot for rendering.
            app.s = Saved::default();
            app.s.saved_targets.push(SavedTarget {
                name: "North Bunker".to_owned(),
                coords: "x98.10, y101.55".to_owned(),
            });
            app.receive(coord(95.62, 104.98), 0.0);
            app.receive(coord(98.10, 101.55), 0.0);
            app.flashed_at = [None; 2];
        }
        harness.run_steps(4);
        if let Ok(path) = std::env::var("WARDOGS_SNAPSHOT") {
            harness.render().expect("render").save(path).expect("save");
        }
    }
}

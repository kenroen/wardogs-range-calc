//! Decorative HUD frame with corner brackets inside the standard window.

use egui::{CornerRadius, Pos2, Rect, Stroke, StrokeKind, pos2, vec2};

use crate::theme;

/// Space the decorative frame takes on each side; the content sits inside it.
pub const FRAME_INSET: f32 = 8.0;

/// Paints the decorative HUD frame around the inside of the window.
pub fn window_frame(ctx: &egui::Context) {
    let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
    let r = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("window-frame")));
    let accent = if focused { theme::ACCENT } else { theme::ACCENT_DIM };

    // Outer hairline and an inner rule, like a stencilled panel.
    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, theme::LINE_STRONG), StrokeKind::Inside);
    let inner = r.shrink(4.0);
    painter.rect_stroke(inner, CornerRadius::ZERO, Stroke::new(1.0, theme::LINE), StrokeKind::Inside);

    // Corner brackets.
    let len = 22.0;
    let bracket = Stroke::new(2.0, accent);
    let b = r.shrink(2.0);
    for (corner, dx, dy) in [
        (b.left_top(), 1.0, 1.0),
        (b.right_top(), -1.0, 1.0),
        (b.left_bottom(), 1.0, -1.0),
        (b.right_bottom(), -1.0, -1.0),
    ] {
        painter.line_segment([corner, corner + vec2(len * dx, 0.0)], bracket);
        painter.line_segment([corner, corner + vec2(0.0, len * dy)], bracket);
        let rivet = corner + vec2(7.0 * dx, 7.0 * dy);
        painter.rect_filled(Rect::from_center_size(rivet, vec2(3.0, 3.0)), CornerRadius::ZERO, accent.gamma_multiply(0.7));
    }

    // Range-finder ticks along the bottom and sides.
    let tick = Stroke::new(1.0, theme::LINE_STRONG);
    let ticks = |a: Pos2, b: Pos2, n: usize, out: egui::Vec2| {
        for k in 1..n {
            let t = k as f32 / n as f32;
            let p = a.lerp(b, t);
            let long = k % 4 == 0;
            painter.line_segment([p, p + out * if long { 5.0 } else { 3.0 }], tick);
        }
    };
    let bottom_y = r.bottom() - 1.0;
    ticks(pos2(r.left() + 40.0, bottom_y), pos2(r.right() - 40.0, bottom_y), 32, vec2(0.0, -1.0));
    ticks(pos2(r.left() + 1.0, r.top() + 50.0), pos2(r.left() + 1.0, r.bottom() - 40.0), 16, vec2(1.0, 0.0));
    ticks(pos2(r.right() - 1.0, r.top() + 50.0), pos2(r.right() - 1.0, r.bottom() - 40.0), 16, vec2(-1.0, 0.0));
}

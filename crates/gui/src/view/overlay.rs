//! Drawn over everything else.

use egui::Color32;

/// A dim overlay while files are being dragged over the window.
pub(super) fn drop_overlay(ctx: &egui::Context) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::unique("drop_overlay"),
    ));
    let rect = ctx.content_rect();
    painter.rect_filled(rect, 0.0, Color32::from_black_alpha(170));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Drop to add",
        egui::FontId::proportional(32.0),
        Color32::WHITE,
    );
}

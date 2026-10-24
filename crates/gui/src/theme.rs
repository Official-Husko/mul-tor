//! Colours and global style.

use egui::Color32;

/// The red of the original Mul-Tor logo.
pub const ACCENT: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);
pub const OK: Color32 = Color32::from_rgb(0x3F, 0xB9, 0x7F);
pub const WARN: Color32 = Color32::from_rgb(0xE8, 0xA3, 0x3D);
pub const ERROR: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);

pub fn apply(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.visuals.selection.bg_fill = ACCENT.gamma_multiply(0.55);
        style.visuals.hyperlink_color = Color32::from_rgb(0x6C, 0xB6, 0xFF);
    });
}

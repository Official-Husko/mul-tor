//! The bar across the top.

use egui::{Align, Layout, RichText, Ui};

use super::Actions;
use crate::app::{Action, MulTorApp};
use crate::theme;

impl MulTorApp {
    pub(super) fn toolbar(&self, ui: &mut Ui, actions: &mut Actions) {
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Mul-Tor").color(theme::ACCENT).strong());
            ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).weak());
            ui.separator();
            if ui.button("Add files…").clicked() {
                actions.push(Action::PickFiles);
            }
            if ui.button("Add folder…").clicked() {
                actions.push(Action::PickFolder);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Settings").clicked() {
                    actions.push(Action::OpenSettings);
                }
                theme_selector(ui);
                if let Some(release) = &self.update {
                    ui.hyperlink_to(
                        RichText::new(format!("Update available: {}", release.name))
                            .color(theme::WARN),
                        &release.url,
                    );
                }
            });
        });
    }
}

/// Light / dark / follow-the-system, as plain text. (egui's own switcher uses
/// emoji, which the bundled fonts do not cover.)
fn theme_selector(ui: &mut Ui) {
    let current = ui.ctx().options(|options| options.theme_preference);
    let mut chosen = current;
    // Called from a right-to-left layout: what is added first ends up rightmost,
    // so the control goes first and its label lands on its left.
    let combo = egui::ComboBox::from_id_salt("theme")
        .selected_text(match current {
            egui::ThemePreference::Dark => "Dark",
            egui::ThemePreference::Light => "Light",
            egui::ThemePreference::System => "System theme",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut chosen, egui::ThemePreference::System, "System theme");
            ui.selectable_value(&mut chosen, egui::ThemePreference::Light, "Light");
            ui.selectable_value(&mut chosen, egui::ThemePreference::Dark, "Dark");
        });
    let label = ui.label("Theme:");
    combo.response.labelled_by(label.id);
    if chosen != current {
        ui.ctx().set_theme(chosen);
    }
}

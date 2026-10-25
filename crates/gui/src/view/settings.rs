//! The settings window frame; its tabs are in `settings_tabs`.

use egui::RichText;

use super::Actions;
use crate::app::{Action, MulTorApp, SettingsTab};
use crate::theme;

impl MulTorApp {
    pub(super) fn settings_window(&mut self, ctx: &egui::Context, actions: &mut Actions) {
        if !self.settings.open {
            return;
        }
        let mut open = true;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.settings.tab, SettingsTab::General, "General");
                    ui.selectable_value(&mut self.settings.tab, SettingsTab::Accounts, "Accounts");
                });
                ui.separator();

                match self.settings.tab {
                    SettingsTab::General => self.settings_general(ui),
                    SettingsTab::Accounts => self.settings_accounts(ui),
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("Save").strong()).fill(theme::ACCENT))
                        .clicked()
                    {
                        actions.push(Action::SaveSettings);
                    }
                    if ui.button("Cancel").clicked() {
                        actions.push(Action::CloseSettings);
                    }
                    ui.label(RichText::new(format!("Saved to {}", self.config_path())).weak());
                });
            });
        if !open {
            self.settings.open = false;
        }
    }

    fn config_path(&self) -> String {
        self.dirs.config_file().display().to_string()
    }
}

//! Drawing. Each function reads state and pushes [`Action`]s; nothing here
//! changes state directly (apart from text boxes bound to it).

mod files;
mod jobs;
mod overlay;
mod providers;
mod queue;
mod settings;
mod settings_tabs;
mod status;
mod toolbar;

use egui::Ui;

use crate::app::{Action, MulTorApp};

type Actions = Vec<Action>;

impl MulTorApp {
    /// The whole window.
    pub fn draw(&mut self, ui: &mut Ui) {
        let mut actions = Actions::new();

        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui, &mut actions));
        egui::Panel::bottom("queue")
            .resizable(true)
            .default_size(280.0)
            .min_size(150.0)
            .show(ui, |ui| self.queue(ui, &mut actions));
        egui::Panel::left("files")
            .resizable(true)
            .default_size(300.0)
            .min_size(200.0)
            .show(ui, |ui| self.files(ui, &mut actions));
        egui::CentralPanel::default().show(ui, |ui| self.providers(ui, &mut actions));

        let ctx = ui.ctx().clone();
        self.settings_window(&ctx, &mut actions);
        overlay::drop_overlay(&ctx);

        for action in actions {
            self.perform(action, &ctx);
        }
    }
}

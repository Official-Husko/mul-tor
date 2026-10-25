//! The bar above the upload list: start/cancel, totals, copy buttons, status.

use egui::{Align, Layout, RichText, Ui};

use super::Actions;
use crate::app::{Action, MulTorApp};
use crate::model::RunState;
use crate::tasks::Level;
use crate::theme;

impl MulTorApp {
    pub(super) fn queue(&self, ui: &mut Ui, actions: &mut Actions) {
        // A resizable panel only honours its default size when its content
        // fills it; otherwise it shrinks to fit (and remembers that size).
        ui.set_min_height(ui.available_height());
        let model = &self.model;
        let files = model.files.len();
        let providers = model.selected.len();

        ui.horizontal(|ui| {
            match model.run {
                RunState::Idle => {
                    let label = format!(
                        "Upload {files} file{} to {providers} provider{}",
                        if files == 1 { "" } else { "s" },
                        if providers == 1 { "" } else { "s" },
                    );
                    let button =
                        egui::Button::new(RichText::new(label).strong()).fill(theme::ACCENT);
                    if ui
                        .add_enabled(model.can_start(), button)
                        .on_disabled_hover_text("Add files and tick at least one provider")
                        .clicked()
                    {
                        actions.push(Action::Start);
                    }
                }
                RunState::Running => {
                    if ui.button("Cancel").clicked() {
                        actions.push(Action::Cancel);
                    }
                }
                RunState::Cancelling => {
                    ui.add_enabled(false, egui::Button::new("Cancelling…"));
                }
            }

            if !model.jobs.is_empty() {
                let counts = model.counts();
                ui.add(
                    egui::ProgressBar::new(model.overall_fraction())
                        .desired_width(220.0)
                        .show_percentage(),
                );
                ui.label(format!(
                    "{} done · {} failed · {} waiting",
                    counts.done,
                    counts.failed,
                    counts.queued + counts.running
                ));
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let has_links = model.has_links();
                if ui
                    .add_enabled(
                        has_links && model.link_format.is_some(),
                        egui::Button::new("Copy links (preset format)"),
                    )
                    .clicked()
                {
                    actions.push(Action::CopyLinks { formatted: true });
                }
                if ui
                    .add_enabled(has_links, egui::Button::new("Copy links"))
                    .clicked()
                {
                    actions.push(Action::CopyLinks { formatted: false });
                }
            });
        });

        if let Some(notice) = self.current_notice() {
            let color = match notice.level {
                Level::Info => ui.visuals().weak_text_color(),
                Level::Error => theme::ERROR,
            };
            ui.label(RichText::new(&notice.text).color(color));
        }
        ui.separator();

        if model.jobs.is_empty() {
            ui.add_space(8.0);
            let message = if self.config.save_links {
                let dir = self
                    .config
                    .links_dir
                    .clone()
                    .unwrap_or_else(|| self.dirs.links_dir());
                format!(
                    "Finished uploads appear here, and their links are saved in {}",
                    dir.display()
                )
            } else {
                "Finished uploads appear here.".to_string()
            };
            ui.label(RichText::new(message).weak());
            return;
        }

        self.jobs_grid(ui, actions);
    }
}

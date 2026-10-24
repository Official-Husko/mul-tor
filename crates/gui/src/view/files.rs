//! The list of files to upload.

use egui::{Align, Layout, RichText, ScrollArea, Ui};
use mul_tor_core::format_bytes;

use super::Actions;
use crate::app::{Action, MulTorApp};

impl MulTorApp {
    pub(super) fn files(&self, ui: &mut Ui, actions: &mut Actions) {
        let model = &self.model;
        ui.horizontal(|ui| {
            ui.heading("Files");
            ui.label(
                RichText::new(format!(
                    "{} · {}",
                    model.files.len(),
                    format_bytes(model.total_size())
                ))
                .weak(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(!model.files.is_empty(), egui::Button::new("Clear"))
                    .clicked()
                {
                    actions.push(Action::ClearFiles);
                }
            });
        });
        ui.separator();

        if model.files.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Drop files here").size(18.0).weak());
                ui.label(RichText::new("or use “Add files…” above").weak());
            });
            return;
        }
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, file) in model.files.iter().enumerate() {
                    ui.horizontal(|ui| {
                        if ui
                            .small_button("×")
                            .on_hover_text("Remove from the list")
                            .clicked()
                        {
                            actions.push(Action::RemoveFile(index));
                        }
                        ui.add(
                            egui::Label::new(format!(
                                "{}  ({})",
                                file.name,
                                format_bytes(file.size)
                            ))
                            .truncate(),
                        )
                        .on_hover_text(file.path.display().to_string());
                    });
                }
            });
    }
}

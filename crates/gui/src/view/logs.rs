//! The log window: what the program has been doing.

use egui::{ComboBox, RichText, ScrollArea};
use tracing::Level;

use super::Actions;
use crate::app::{Action, LogDetail, MulTorApp};
use crate::theme;

/// Records fetched per frame; the buffer itself holds more.
pub const VISIBLE_RECORDS: usize = 500;

impl MulTorApp {
    pub(super) fn logs_window(&mut self, ctx: &egui::Context, actions: &mut Actions) {
        if !self.log_window.open {
            return;
        }
        let mut open = true;
        let detail = self.log_window.detail;
        let mut chosen = detail;
        egui::Window::new("Log")
            .open(&mut open)
            .default_size([780.0, 340.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let label = ui.label("Show:");
                    let combo = ComboBox::from_id_salt("log_detail")
                        .selected_text(detail.label())
                        .show_ui(ui, |ui| {
                            for option in LogDetail::ALL {
                                ui.selectable_value(&mut chosen, option, option.label());
                            }
                        });
                    combo.response.labelled_by(label.id);
                    if ui.button("Copy").clicked() {
                        actions.push(Action::CopyLog);
                    }
                    ui.label(
                        RichText::new(format!("Files are in {}", self.dirs.log_dir().display()))
                            .weak(),
                    );
                });
                ui.separator();

                let records = self.logs.tail(VISIBLE_RECORDS);
                ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for record in records.iter().filter(|r| detail.shows(r.level)) {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(record.time.format("%H:%M:%S%.3f").to_string())
                                        .monospace()
                                        .weak(),
                                );
                                ui.label(
                                    RichText::new(format!("{:>5}", record.level))
                                        .monospace()
                                        .color(level_color(record.level)),
                                );
                                let mut text = record.message.clone();
                                if !record.fields.is_empty() {
                                    text.push_str("  ");
                                    text.push_str(&record.fields_text());
                                }
                                ui.label(text);
                            });
                        }
                    });
            });
        self.log_window.detail = chosen;
        if !open {
            self.log_window.open = false;
        }
    }
}

fn level_color(level: Level) -> egui::Color32 {
    match level {
        Level::ERROR => theme::ERROR,
        Level::WARN => theme::WARN,
        Level::INFO => theme::ACCENT,
        _ => egui::Color32::GRAY,
    }
}

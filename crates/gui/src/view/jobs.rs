//! One row per upload.

use egui::{RichText, ScrollArea, Ui};
use mul_tor_core::format_bytes;

use super::Actions;
use crate::app::{Action, MulTorApp};
use crate::model::JobState;
use crate::theme;

impl MulTorApp {
    pub(super) fn jobs_grid(&self, ui: &mut Ui, actions: &mut Actions) {
        let model = &self.model;
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("jobs")
                    .num_columns(4)
                    .striped(true)
                    .min_col_width(70.0)
                    .spacing([14.0, 6.0])
                    .show(ui, |ui| {
                        for job in &model.jobs {
                            match &job.state {
                                JobState::Queued => ui.label(RichText::new("waiting").weak()),
                                JobState::Running(_) => ui.spinner(),
                                JobState::Done { .. } => {
                                    ui.label(RichText::new("done").color(theme::OK))
                                }
                                JobState::Failed { .. } => {
                                    ui.label(RichText::new("failed").color(theme::ERROR))
                                }
                                JobState::Cancelled => ui.label(RichText::new("cancelled").weak()),
                            };
                            ui.scope(|ui| {
                                ui.set_width(240.0);
                                ui.add(egui::Label::new(&job.file).truncate())
                                    .on_hover_text(&job.file);
                            });
                            ui.label(RichText::new(job.provider).strong());

                            match &job.state {
                                JobState::Queued => {
                                    ui.label(RichText::new("waiting").weak());
                                }
                                JobState::Running(progress) => {
                                    let fraction = if progress.total == 0 {
                                        0.0
                                    } else {
                                        progress.sent as f32 / progress.total as f32
                                    };
                                    let mut text = format!(
                                        "{} / {}",
                                        format_bytes(progress.sent),
                                        format_bytes(progress.total)
                                    );
                                    if !progress.message.is_empty() {
                                        text = format!("{} · {text}", progress.message);
                                    }
                                    ui.add(
                                        egui::ProgressBar::new(fraction)
                                            .desired_width(320.0)
                                            .text(text),
                                    );
                                }
                                JobState::Done { url } => {
                                    ui.horizontal(|ui| {
                                        if ui.small_button("Copy").clicked() {
                                            actions.push(Action::CopyText(url.clone()));
                                        }
                                        if ui.small_button("Open").clicked() {
                                            actions.push(Action::OpenUrl(url.clone()));
                                        }
                                        ui.add(
                                            egui::Label::new(RichText::new(url).color(theme::OK))
                                                .truncate(),
                                        );
                                    });
                                }
                                JobState::Failed { message, .. } => {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(message).color(theme::ERROR),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(message);
                                }
                                JobState::Cancelled => {
                                    ui.label(RichText::new("cancelled").weak());
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
    }
}

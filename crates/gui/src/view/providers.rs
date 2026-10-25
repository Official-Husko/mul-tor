//! The provider checklist: search, presets, and one row per provider.

use egui::{RichText, ScrollArea, Ui};

use super::{Actions, status};
use crate::app::{Action, MulTorApp};
use crate::model::matches_filter;

impl MulTorApp {
    pub(super) fn providers(&self, ui: &mut Ui, actions: &mut Actions) {
        ui.horizontal(|ui| {
            ui.heading("Providers");
            ui.label(RichText::new(format!("{} selected", self.model.selected.len())).weak());
        });
        self.provider_controls(ui, actions);
        ui.separator();
        self.provider_list(ui, actions);
    }

    fn provider_controls(&self, ui: &mut Ui, actions: &mut Actions) {
        ui.horizontal_wrapped(|ui| {
            let mut filter = self.model.provider_filter.clone();
            let response = ui.add(
                egui::TextEdit::singleline(&mut filter)
                    .hint_text("Search providers")
                    .desired_width(160.0),
            );
            if response.changed() {
                actions.push(Action::SetFilter(filter));
            }

            let preset_label = self
                .model
                .preset
                .as_deref()
                .and_then(|stem| self.presets.find(stem))
                .map_or("none".to_string(), |p| {
                    if p.name.is_empty() {
                        p.identifier.clone()
                    } else {
                        p.name.clone()
                    }
                });
            let preset_name = ui.label("Preset:");
            let preset_combo = egui::ComboBox::from_id_salt("preset")
                .selected_text(preset_label)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.model.preset.is_none(), "No preset")
                        .clicked()
                    {
                        actions.push(Action::ApplyPreset(None));
                    }
                    for (stem, preset) in self.presets.iter() {
                        let name = if preset.name.is_empty() {
                            stem
                        } else {
                            &preset.name
                        };
                        let chosen = self.model.preset.as_deref() == Some(stem);
                        if ui.selectable_label(chosen, name).clicked() {
                            actions.push(Action::ApplyPreset(Some(stem.to_string())));
                        }
                    }
                });
            preset_combo.response.labelled_by(preset_name.id);

            if ui
                .button("All")
                .on_hover_text("Tick every provider shown")
                .clicked()
            {
                actions.push(Action::SelectAllProviders);
            }
            if ui.button("None").clicked() {
                actions.push(Action::SelectNoProviders);
            }
            if ui
                .add_enabled(
                    !self.model.checking,
                    egui::Button::new("Check availability"),
                )
                .on_hover_text("Ping every provider and mark the ones that do not answer")
                .clicked()
            {
                actions.push(Action::CheckAvailability);
            }
            if self.model.checking {
                ui.spinner();
            }
        });
    }

    fn provider_list(&self, ui: &mut Ui, actions: &mut Actions) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("providers")
                    .num_columns(5)
                    .striped(true)
                    .min_col_width(96.0)
                    .spacing([18.0, 6.0])
                    .show(ui, |ui| {
                        for row in &self.rows {
                            if !matches_filter(&*row.provider, &self.model.provider_filter) {
                                continue;
                            }
                            let info = row.provider.info();
                            let lacks = self.lacks_credentials(&row.provider);

                            let mut on = self.model.selected.contains(&row.id);
                            let changed = ui
                                .add_enabled_ui(!lacks, |ui| ui.checkbox(&mut on, info.name))
                                .inner
                                .changed();
                            if changed {
                                actions.push(Action::ToggleProvider {
                                    id: row.id.clone(),
                                    on,
                                });
                            }

                            ui.label(
                                RichText::new(
                                    info.max_size
                                        .map_or("no limit".to_string(), |l| l.to_string()),
                                )
                                .weak(),
                            );

                            status::account_cell(ui, info, lacks);
                            status::health_cell(ui, self.model.health.get(&row.id));
                            status::reliability_cell(ui, info.reliability);
                            ui.end_row();
                        }
                    });
            });
    }
}

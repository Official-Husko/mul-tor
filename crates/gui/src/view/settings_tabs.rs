//! The General and Accounts tabs of the settings window.

use egui::{RichText, ScrollArea, Ui};

use crate::app::MulTorApp;

impl MulTorApp {
    pub(super) fn settings_general(&mut self, ui: &mut Ui) {
        let draft = &mut self.settings.draft;
        egui::Grid::new("general")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                let label = ui.label("Uploads at once");
                ui.add(egui::DragValue::new(&mut draft.parallel_uploads).range(1..=16))
                    .labelled_by(label.id);
                ui.end_row();

                ui.label("Save links to a file");
                ui.checkbox(&mut draft.save_links, "file_links.txt");
                ui.end_row();

                ui.label("Check providers at startup");
                ui.checkbox(
                    &mut draft.check_availability,
                    "mark providers that do not answer",
                );
                ui.end_row();

                ui.label("Look for updates");
                ui.checkbox(&mut draft.check_for_updates, "tells you; never installs");
                ui.end_row();

                ui.label("Browser user agents");
                ui.checkbox(
                    &mut draft.random_user_agent,
                    "rotate instead of identifying as Mul-Tor",
                );
                ui.end_row();

                ui.label("Public proxy pool");
                ui.checkbox(
                    &mut draft.use_proxies,
                    "slow, and the proxy can read your traffic",
                );
                ui.end_row();

                ui.label("Your own proxy");
                let mut proxy = draft.proxy.clone().unwrap_or_default();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut proxy)
                            .hint_text("socks5h://127.0.0.1:9050")
                            .desired_width(260.0),
                    )
                    .changed()
                {
                    draft.proxy = (!proxy.trim().is_empty()).then_some(proxy);
                }
                ui.end_row();
            });
        ui.label(RichText::new("Changes apply to the next upload.").weak());
    }

    pub(super) fn settings_accounts(&mut self, ui: &mut Ui) {
        ui.label(
            RichText::new("Providers that need an account. Leave blank to keep them hidden.")
                .weak(),
        );
        ui.checkbox(&mut self.settings.reveal_secrets, "Show secrets");
        ui.add_space(4.0);

        let reveal = self.settings.reveal_secrets;
        ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
            for row in &self.rows {
                let info = row.provider.info();
                if info.credentials.is_empty() {
                    continue;
                }
                egui::CollapsingHeader::new(RichText::new(info.name).strong())
                    .default_open(true)
                    .show(ui, |ui| {
                        if let Some(url) = info.credentials_url {
                            ui.hyperlink_to("Where do I get these?", url);
                        }
                        egui::Grid::new(("creds", &row.id))
                            .num_columns(2)
                            .show(ui, |ui| {
                                for field in info.credentials {
                                    let label = ui.label(field.label);
                                    let buffer = self
                                        .settings
                                        .buffers
                                        .entry((row.id.clone(), field.key.to_string()))
                                        .or_default();
                                    ui.add(
                                        egui::TextEdit::singleline(buffer)
                                            .password(field.secret && !reveal)
                                            .desired_width(260.0),
                                    )
                                    .labelled_by(label.id);
                                    ui.end_row();
                                }
                            });
                    });
            }
        });
    }
}

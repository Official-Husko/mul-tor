//! Drawing. Each function reads state and pushes [`Action`]s; nothing here
//! changes state directly (apart from text boxes bound to it).

use egui::{Align, Color32, Layout, RichText, ScrollArea, Ui};
use mul_tor_core::availability::Health;
use mul_tor_core::{Auth, Reliability, format_bytes};

use crate::app::{Action, MulTorApp, SettingsTab};
use crate::model::{JobState, RunState, matches_filter};
use crate::tasks::Level;
use crate::theme;

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
        drop_overlay(&ctx);

        for action in actions {
            self.perform(action, &ctx);
        }
    }

    fn toolbar(&self, ui: &mut Ui, actions: &mut Actions) {
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

    fn files(&self, ui: &mut Ui, actions: &mut Actions) {
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

    fn providers(&self, ui: &mut Ui, actions: &mut Actions) {
        ui.horizontal(|ui| {
            ui.heading("Providers");
            ui.label(RichText::new(format!("{} selected", self.model.selected.len())).weak());
        });
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
        ui.separator();

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

                            match (info.auth, lacks) {
                                (Auth::None, _) => ui.label(""),
                                (_, true) => ui
                                    .label(RichText::new("needs account").color(theme::WARN))
                                    .on_hover_text(
                                        "Add your credentials in Settings, on the Accounts tab",
                                    ),
                                (_, false) => {
                                    ui.label(RichText::new("account set").color(theme::OK))
                                }
                            };

                            match self.model.health.get(&row.id) {
                                Some(Health::Up) => {
                                    ui.label(RichText::new("online").color(theme::OK))
                                }
                                Some(Health::Down(reason)) => ui
                                    .label(RichText::new("offline").color(theme::ERROR))
                                    .on_hover_text(reason),
                                None => ui.label(""),
                            };

                            match info.reliability {
                                Reliability::Unverified => {
                                    ui.label(RichText::new("unverified").weak()).on_hover_text(
                                        "Nobody has tried this against the live service yet",
                                    )
                                }
                                Reliability::Working { checked } => ui
                                    .label(RichText::new("working").color(theme::OK))
                                    .on_hover_text(format!("Seen working on {checked}")),
                                Reliability::Degraded(reason) => ui
                                    .label(RichText::new("degraded").color(theme::WARN))
                                    .on_hover_text(reason),
                                Reliability::Broken(reason) => ui
                                    .label(RichText::new("broken").color(theme::ERROR))
                                    .on_hover_text(format!(
                                        "{reason} (it is left out of Select all and presets)"
                                    )),
                            };
                            ui.end_row();
                        }
                    });
            });
    }

    fn queue(&self, ui: &mut Ui, actions: &mut Actions) {
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

    fn settings_window(&mut self, ctx: &egui::Context, actions: &mut Actions) {
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

    fn settings_general(&mut self, ui: &mut Ui) {
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

    fn settings_accounts(&mut self, ui: &mut Ui) {
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

    fn config_path(&self) -> String {
        self.dirs.config_file().display().to_string()
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

/// A dim overlay while files are being dragged over the window.
fn drop_overlay(ctx: &egui::Context) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::unique("drop_overlay"),
    ));
    let rect = ctx.content_rect();
    painter.rect_filled(rect, 0.0, Color32::from_black_alpha(170));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Drop to add",
        egui::FontId::proportional(32.0),
        Color32::WHITE,
    );
}

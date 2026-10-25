//! Turning messages into changes and commands.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;

use super::overlay::{Overlay, Preflight};
use super::{Model, Screen};
use crate::tui::msg::{Cmd, Level, Msg, Plan};

impl Model {
    pub fn update(&mut self, msg: Msg) -> Vec<Cmd> {
        match msg {
            Msg::Key(key) => self.on_key(key),
            Msg::Paste(text) => {
                self.on_paste(&text);
                Vec::new()
            }
            Msg::Tick => {
                self.tick += 1;
                if self
                    .notice
                    .as_ref()
                    .is_some_and(|n| n.expires_at <= self.tick)
                {
                    self.notice = None;
                }
                Vec::new()
            }
            Msg::Engine(event) => {
                self.upload.apply(event);
                Vec::new()
            }
            Msg::UploadFinished => {
                self.on_upload_finished();
                Vec::new()
            }
            Msg::Availability(results) => {
                self.providers.set_probe_results(results);
                Vec::new()
            }
            Msg::UpdateFound(release) => {
                self.notify(Level::Info, format!("Update available: {}", release.name));
                self.update = Some(release);
                Vec::new()
            }
            Msg::Notice(level, text) => {
                self.notify(level, text);
                Vec::new()
            }
        }
    }

    fn on_paste(&mut self, text: &str) {
        let notice = match self.screen {
            Screen::Files if self.overlay.is_none() => self.files.paste(text),
            Screen::Providers if self.providers.filtering => {
                self.providers.filter.insert_str(text);
                self.providers.filter_changed();
                None
            }
            _ => None,
        };
        self.notify_opt(notice);
    }

    fn on_key(&mut self, key: KeyEvent) -> Vec<Cmd> {
        if key.kind == KeyEventKind::Release {
            return Vec::new();
        }
        let ctrl_c =
            key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
        if self.overlay.is_some() {
            return self.on_overlay_key(key, ctrl_c);
        }
        if ctrl_c {
            return self.interrupt();
        }
        if !self.typing() {
            match key.code {
                KeyCode::Char('?') | KeyCode::F(1) => {
                    self.overlay = Some(Overlay::Help);
                    return Vec::new();
                }
                KeyCode::Char('L') | KeyCode::F(3) => {
                    self.show_logs = !self.show_logs;
                    return Vec::new();
                }
                _ => {}
            }
        }
        match self.screen {
            Screen::Files => self.on_files_key(key),
            Screen::Providers => self.on_providers_key(key),
            Screen::Upload => self.on_upload_key(key),
            Screen::Results => self.on_results_key(key),
        }
    }

    /// Ctrl+C: stop a running upload (twice to leave at once), otherwise ask to quit.
    pub(super) fn interrupt(&mut self) -> Vec<Cmd> {
        if self.screen == Screen::Upload && !self.upload.is_finished() {
            if self.upload.cancelling {
                self.exit_code = 130;
                self.quit = true;
                return vec![Cmd::Quit];
            }
            self.upload.cancelling = true;
            self.notify(Level::Warn, "Cancelling... press Ctrl+C again to quit now");
            return vec![Cmd::CancelUpload];
        }
        self.overlay = Some(Overlay::Quit);
        Vec::new()
    }

    fn on_overlay_key(&mut self, key: KeyEvent, ctrl_c: bool) -> Vec<Cmd> {
        let Some(overlay) = self.overlay.as_mut() else {
            return Vec::new();
        };
        match overlay {
            Overlay::Help => self.overlay = None,
            Overlay::Quit => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => return self.quit_now(),
                KeyCode::Char('n') | KeyCode::Esc => self.overlay = None,
                _ if ctrl_c => return self.quit_now(),
                _ => {}
            },
            Overlay::Confirm(_) => match key.code {
                KeyCode::Enter | KeyCode::Char('y') => return self.start_upload(),
                KeyCode::Esc | KeyCode::Char('n') => self.overlay = None,
                _ if ctrl_c => self.overlay = None,
                _ => {}
            },
            Overlay::Presets(list) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => list.select_previous(),
                KeyCode::Down | KeyCode::Char('j') => list.select_next(),
                KeyCode::Enter => {
                    let choice = list.selected().unwrap_or(0);
                    self.overlay = None;
                    self.apply_preset_choice(choice);
                }
                KeyCode::Esc | KeyCode::Char('q') => self.overlay = None,
                _ if ctrl_c => self.overlay = None,
                _ => {}
            },
        }
        Vec::new()
    }

    fn quit_now(&mut self) -> Vec<Cmd> {
        self.quit = true;
        vec![Cmd::Quit]
    }

    fn apply_preset_choice(&mut self, choice: usize) {
        let index = choice.checked_sub(1);
        self.providers.apply_preset(index);
        let notice = match index.and_then(|i| self.providers.presets.get(i)) {
            Some(preset) if !preset.notes.is_empty() => Some((
                Level::Info,
                format!("{}: {}", preset.title, preset.notes.join("; ")),
            )),
            Some(preset) => Some((
                Level::Info,
                format!("{}: ticked {} providers", preset.title, preset.ids.len()),
            )),
            None => Some((Level::Info, "No preset; the ticks were cleared".to_string())),
        };
        self.notify_opt(notice);
    }

    pub(super) fn open_presets(&mut self) {
        if self.providers.presets.is_empty() {
            self.notify(Level::Warn, "No preset has providers you can use right now");
            return;
        }
        let at = self.providers.preset.map_or(0, |index| index + 1);
        self.overlay = Some(Overlay::Presets(
            ListState::default().with_selected(Some(at)),
        ));
    }

    pub(super) fn open_confirm(&mut self) {
        let chosen: Vec<_> = self
            .providers
            .rows
            .iter()
            .filter(|row| self.providers.is_selected(row))
            .collect();
        let report = Preflight::compute(&self.files.basket, &chosen, &self.settings.credentials);
        self.overlay = Some(Overlay::Confirm(Box::new(report)));
    }

    fn start_upload(&mut self) -> Vec<Cmd> {
        self.overlay = None;
        self.upload.begin();
        self.screen = Screen::Upload;
        let plan = Plan {
            files: self.files.files(),
            providers: self.providers.selected_providers(),
            link_format: self.providers.link_format(),
            parallel: self.settings.parallel,
            save_links: self.settings.save_links,
            links_dir: self.settings.links_dir.clone(),
        };
        vec![Cmd::StartUpload(plan)]
    }

    fn on_upload_finished(&mut self) {
        self.upload.finish();
        self.screen = Screen::Results;
        self.upload
            .list
            .select((!self.upload.jobs.is_empty()).then_some(0));
        let counts = self.upload.counts();
        self.exit_code = if self.upload.cancelling {
            130
        } else if counts.failed > 0 {
            1
        } else {
            0
        };
        let (level, text) = if counts.failed + counts.cancelled == 0 {
            (Level::Info, format!("All {} uploads finished", counts.ok))
        } else {
            (
                Level::Warn,
                format!(
                    "{} uploaded, {} failed, {} cancelled",
                    counts.ok, counts.failed, counts.cancelled
                ),
            )
        };
        self.notify(level, text);
    }
}

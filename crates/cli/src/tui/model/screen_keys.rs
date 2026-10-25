//! Keys that belong to one screen, after the global ones were checked.

use crossterm::event::{KeyCode, KeyEvent};
use mul_tor_core::linklog::render_links;

use super::files_keys::FilesOutcome;
use super::providers_keys::ProvidersOutcome;
use super::{Model, Overlay, Screen};
use crate::tui::msg::{Cmd, Level};

impl Model {
    pub(super) fn on_files_key(&mut self, key: KeyEvent) -> Vec<Cmd> {
        let result = self.files.handle_key(key);
        self.notify_opt(result.notice);
        match result.outcome {
            FilesOutcome::Stay => {}
            FilesOutcome::Next => self.screen = Screen::Providers,
            FilesOutcome::Quit => self.overlay = Some(Overlay::Quit),
        }
        Vec::new()
    }

    pub(super) fn on_providers_key(&mut self, key: KeyEvent) -> Vec<Cmd> {
        let result = self.providers.handle_key(key);
        self.notify_opt(result.notice);
        match result.outcome {
            ProvidersOutcome::Stay => {}
            ProvidersOutcome::Back => self.screen = Screen::Files,
            ProvidersOutcome::Confirm => self.open_confirm(),
            ProvidersOutcome::Presets => self.open_presets(),
            ProvidersOutcome::Quit => self.overlay = Some(Overlay::Quit),
        }
        Vec::new()
    }

    pub(super) fn on_upload_key(&mut self, key: KeyEvent) -> Vec<Cmd> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.upload.list.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.upload.list.select_next(),
            KeyCode::PageUp => self.upload.list.scroll_up_by(10),
            KeyCode::PageDown => self.upload.list.scroll_down_by(10),
            KeyCode::Home | KeyCode::Char('g') => self.upload.list.select_first(),
            KeyCode::End | KeyCode::Char('G') => self.upload.list.select_last(),
            KeyCode::Esc | KeyCode::Char('q') if !self.upload.cancelling => {
                return self.interrupt();
            }
            _ => {}
        }
        Vec::new()
    }

    pub(super) fn on_results_key(&mut self, key: KeyEvent) -> Vec<Cmd> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.upload.list.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.upload.list.select_next(),
            KeyCode::PageUp => self.upload.list.scroll_up_by(10),
            KeyCode::PageDown => self.upload.list.scroll_down_by(10),
            KeyCode::Home | KeyCode::Char('g') => self.upload.list.select_first(),
            KeyCode::End | KeyCode::Char('G') => self.upload.list.select_last(),
            KeyCode::Char('c') => return self.copy_links(None),
            KeyCode::Char('f') => match self.providers.link_format() {
                Some(template) => return self.copy_links(Some(template)),
                None => self.notify(
                    Level::Warn,
                    "No preset was used, so there is no formatted list",
                ),
            },
            KeyCode::Char('b') => self.screen = Screen::Providers,
            KeyCode::Char('n') => {
                self.files.clear();
                self.upload = Default::default();
                self.screen = Screen::Files;
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                self.quit = true;
                return vec![Cmd::Quit];
            }
            _ => {}
        }
        Vec::new()
    }

    fn copy_links(&mut self, template: Option<String>) -> Vec<Cmd> {
        let links = self.upload.links();
        if links.is_empty() {
            self.notify(
                Level::Warn,
                "Nothing was uploaded, so there is nothing to copy",
            );
            return Vec::new();
        }
        let text = render_links(links, template.as_deref());
        vec![Cmd::Copy(text)]
    }
}

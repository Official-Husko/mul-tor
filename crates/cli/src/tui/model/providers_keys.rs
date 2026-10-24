//! Keys on the provider-choosing step.

use crossterm::event::{KeyCode, KeyEvent};

use super::providers::ProvidersState;
use crate::tui::msg::Level;

pub enum ProvidersOutcome {
    Stay,
    Back,
    /// Ask to start uploading.
    Confirm,
    Presets,
    Quit,
}

pub struct ProvidersResult {
    pub outcome: ProvidersOutcome,
    pub notice: Option<(Level, String)>,
}

impl ProvidersResult {
    fn stay() -> Self {
        Self {
            outcome: ProvidersOutcome::Stay,
            notice: None,
        }
    }

    fn go(outcome: ProvidersOutcome) -> Self {
        Self {
            outcome,
            notice: None,
        }
    }
}

const PAGE: u16 = 10;

impl ProvidersState {
    pub fn handle_key(&mut self, key: KeyEvent) -> ProvidersResult {
        if self.filtering {
            return self.filter_key(key);
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.list.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.list.select_next(),
            KeyCode::PageUp => self.list.scroll_up_by(PAGE),
            KeyCode::PageDown => self.list.scroll_down_by(PAGE),
            KeyCode::Home | KeyCode::Char('g') => self.list.select_first(),
            KeyCode::End | KeyCode::Char('G') => self.list.select_last(),
            KeyCode::Char(' ') => {
                let notice = self.toggle_current().err().map(|text| (Level::Warn, text));
                return ProvidersResult {
                    outcome: ProvidersOutcome::Stay,
                    notice,
                };
            }
            KeyCode::Char('a') => {
                let count = self.select_visible();
                let text = format!(
                    "Ticked {count}; left out the ones that are broken, offline or need an account"
                );
                return ProvidersResult {
                    outcome: ProvidersOutcome::Stay,
                    notice: Some((Level::Info, text)),
                };
            }
            KeyCode::Char('d') => self.clear_selection(),
            KeyCode::Char('/') => self.filtering = true,
            KeyCode::Char('p') => return ProvidersResult::go(ProvidersOutcome::Presets),
            KeyCode::Enter | KeyCode::Char('u') => return self.confirm(),
            KeyCode::Esc if !self.filter.is_empty() => {
                self.filter.clear();
                self.filter_changed();
            }
            KeyCode::Esc | KeyCode::Left | KeyCode::Backspace | KeyCode::Char('b') => {
                return ProvidersResult::go(ProvidersOutcome::Back);
            }
            KeyCode::Char('q') => return ProvidersResult::go(ProvidersOutcome::Quit),
            _ => {}
        }
        ProvidersResult::stay()
    }

    fn confirm(&self) -> ProvidersResult {
        if self.selected.is_empty() {
            return ProvidersResult {
                outcome: ProvidersOutcome::Stay,
                notice: Some((
                    Level::Warn,
                    "Tick at least one provider (Space)".to_string(),
                )),
            };
        }
        ProvidersResult::go(ProvidersOutcome::Confirm)
    }

    fn filter_key(&mut self, key: KeyEvent) -> ProvidersResult {
        match key.code {
            KeyCode::Esc => {
                self.filter.clear();
                self.filtering = false;
                self.filter_changed();
            }
            KeyCode::Enter => self.filtering = false,
            KeyCode::Up => self.list.select_previous(),
            KeyCode::Down => self.list.select_next(),
            _ => {
                if self.filter.handle_key(key) {
                    self.filter_changed();
                }
            }
        }
        ProvidersResult::stay()
    }
}

//! Keys and pasted text on the file-choosing step.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

use super::files::{FilesState, Pane};
use crate::paths::parse_paths;
use crate::tui::msg::Level;

pub enum FilesOutcome {
    Stay,
    /// Go on to choosing providers.
    Next,
    Quit,
}

pub struct FilesResult {
    pub outcome: FilesOutcome,
    pub notice: Option<(Level, String)>,
}

impl FilesResult {
    fn stay() -> Self {
        Self {
            outcome: FilesOutcome::Stay,
            notice: None,
        }
    }

    fn with(notice: Option<(Level, String)>) -> Self {
        Self {
            outcome: FilesOutcome::Stay,
            notice,
        }
    }
}

const PAGE: usize = 10;

impl FilesState {
    pub fn handle_key(&mut self, key: KeyEvent) -> FilesResult {
        match self.pane {
            Pane::Path => self.path_key(key),
            Pane::Browser => self.browser_key(key),
            Pane::Basket => self.basket_key(key),
        }
    }

    /// Pasted text is typed into the path field, or, anywhere else, taken as
    /// paths to add (that is what dropping a file on the terminal sends).
    pub fn paste(&mut self, text: &str) -> Option<(Level, String)> {
        if self.pane == Pane::Path {
            self.path_input.insert_str(text);
            return None;
        }
        let paths = parse_paths(text);
        if paths.is_empty() {
            return None;
        }
        self.add_paths(&paths).notice()
    }

    fn next(&self) -> FilesResult {
        if self.basket.is_empty() {
            FilesResult::with(Some((
                Level::Warn,
                "Add at least one file first".to_string(),
            )))
        } else {
            FilesResult {
                outcome: FilesOutcome::Next,
                notice: None,
            }
        }
    }

    fn browser_key(&mut self, key: KeyEvent) -> FilesResult {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.browser.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.browser.select_next(),
            KeyCode::PageUp => self.browser.scroll_up_by(PAGE as u16),
            KeyCode::PageDown => self.browser.scroll_down_by(PAGE as u16),
            KeyCode::Home | KeyCode::Char('g') => self.browser.select_first(),
            KeyCode::End | KeyCode::Char('G') => self.browser.select_last(),
            KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') => self.go_up(),
            KeyCode::Enter | KeyCode::Right => return self.open_or_add(),
            KeyCode::Char(' ') => return FilesResult::with(self.toggle_current()),
            KeyCode::Char('a') => {
                let files: Vec<PathBuf> = self
                    .items
                    .iter()
                    .filter(|item| !item.is_dir)
                    .map(|item| item.path.clone())
                    .collect();
                return FilesResult::with(self.add_paths(&files).notice());
            }
            KeyCode::Char('/') | KeyCode::Char('p') => self.pane = Pane::Path,
            KeyCode::Char('~') => {
                if let Some(dirs) = directories::BaseDirs::new() {
                    self.open_dir(dirs.home_dir().to_path_buf());
                }
            }
            KeyCode::Char('.') => {
                self.show_hidden = !self.show_hidden;
                self.reload();
            }
            KeyCode::Char('r') => self.recursive = !self.recursive,
            KeyCode::Tab | KeyCode::BackTab if !self.basket.is_empty() => self.pane = Pane::Basket,
            KeyCode::Char('n') => return self.next(),
            KeyCode::Esc | KeyCode::Char('q') => {
                return FilesResult {
                    outcome: FilesOutcome::Quit,
                    notice: None,
                };
            }
            _ => {}
        }
        FilesResult::stay()
    }

    fn open_or_add(&mut self) -> FilesResult {
        let Some(item) = self.current_item().cloned() else {
            return FilesResult::stay();
        };
        if item.is_dir {
            self.open_dir(item.path);
            FilesResult::stay()
        } else {
            FilesResult::with(self.add_paths(&[item.path]).notice())
        }
    }

    /// Space: add or remove the file under the cursor; for a folder, add its files.
    fn toggle_current(&mut self) -> Option<(Level, String)> {
        let item = self.current_item()?.clone();
        if item.is_parent {
            return None;
        }
        if !item.is_dir && self.contains(&item.path) {
            let at = self
                .basket
                .iter()
                .position(|entry| entry.path == item.path)?;
            self.basket_cursor.select(Some(at));
            self.remove_current();
            return Some((Level::Info, "Removed from the list".to_string()));
        }
        self.add_paths(&[item.path]).notice()
    }

    fn basket_key(&mut self, key: KeyEvent) -> FilesResult {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.basket_cursor.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.basket_cursor.select_next(),
            KeyCode::PageUp => self.basket_cursor.scroll_up_by(PAGE as u16),
            KeyCode::PageDown => self.basket_cursor.scroll_down_by(PAGE as u16),
            KeyCode::Home | KeyCode::Char('g') => self.basket_cursor.select_first(),
            KeyCode::End | KeyCode::Char('G') => self.basket_cursor.select_last(),
            KeyCode::Char('x') | KeyCode::Delete | KeyCode::Backspace => {
                self.remove_current();
                if self.basket.is_empty() {
                    self.pane = Pane::Browser;
                }
            }
            KeyCode::Char('c') => {
                self.clear();
                self.pane = Pane::Browser;
                return FilesResult::with(Some((Level::Info, "List cleared".to_string())));
            }
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc => self.pane = Pane::Browser,
            KeyCode::Enter | KeyCode::Char('n') => return self.next(),
            KeyCode::Char('q') => {
                return FilesResult {
                    outcome: FilesOutcome::Quit,
                    notice: None,
                };
            }
            _ => {}
        }
        FilesResult::stay()
    }

    fn path_key(&mut self, key: KeyEvent) -> FilesResult {
        match key.code {
            KeyCode::Esc | KeyCode::Tab => self.pane = Pane::Browser,
            KeyCode::Enter => return FilesResult::with(self.submit_path()),
            _ => {
                self.path_input.handle_key(key);
            }
        }
        FilesResult::stay()
    }

    /// Enter in the path field: one folder opens in the browser, anything
    /// else is added.
    fn submit_path(&mut self) -> Option<(Level, String)> {
        let paths = parse_paths(&self.path_input.take());
        self.pane = Pane::Browser;
        match paths.as_slice() {
            [] => None,
            [one] if one.is_dir() => {
                self.open_dir(one.clone());
                None
            }
            many => self.add_paths(many).notice(),
        }
    }
}

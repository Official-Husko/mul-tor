//! The first step: choosing what to upload. A folder browser on the left, the
//! list of files that will be uploaded (the basket) on the right.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use mul_tor_core::files::scan_inputs;
use ratatui::widgets::ListState;

use super::browser::{DirItem, list_dir};
use crate::tui::input::TextInput;
use crate::tui::msg::Level;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Browser,
    Basket,
    /// Typing a path.
    Path,
}

/// What adding paths to the basket did.
#[derive(Debug, Default)]
pub struct AddReport {
    pub added: usize,
    pub duplicates: usize,
    pub problems: Vec<(PathBuf, String)>,
}

impl AddReport {
    /// One line for the status bar, or `None` when nothing happened.
    pub fn notice(&self) -> Option<(Level, String)> {
        if let Some((path, reason)) = self.problems.first() {
            let more = self.problems.len().saturating_sub(1);
            let tail = if more > 0 {
                format!(" (+{more} more)")
            } else {
                String::new()
            };
            return Some((
                Level::Warn,
                format!("Cannot use {}: {reason}{tail}", path.display()),
            ));
        }
        match (self.added, self.duplicates) {
            (0, 0) => None,
            (0, _) => Some((Level::Info, "Already in the list".to_string())),
            (added, 0) => Some((Level::Info, format!("Added {}", count_files(added)))),
            (added, dup) => Some((
                Level::Info,
                format!("Added {} ({dup} already in the list)", count_files(added)),
            )),
        }
    }
}

fn count_files(n: usize) -> String {
    crate::tui::format::count(n, "file", "files")
}

pub struct FilesState {
    pub dir: PathBuf,
    pub items: Vec<DirItem>,
    pub browser: ListState,
    pub basket: Vec<FileEntry>,
    pub basket_cursor: ListState,
    pub pane: Pane,
    pub path_input: TextInput,
    pub show_hidden: bool,
    pub recursive: bool,
    /// Why the current folder could not be read.
    pub browse_error: Option<String>,
    /// Canonical paths of everything in the basket.
    known: HashSet<PathBuf>,
}

impl FilesState {
    pub fn new(start: PathBuf) -> Self {
        let mut state = Self {
            dir: start,
            items: Vec::new(),
            browser: ListState::default(),
            basket: Vec::new(),
            basket_cursor: ListState::default(),
            pane: Pane::Browser,
            path_input: TextInput::default(),
            show_hidden: false,
            recursive: false,
            browse_error: None,
            known: HashSet::new(),
        };
        state.reload();
        state
    }

    /// Read the current folder again; the cursor lands on the first real entry.
    pub fn reload(&mut self) {
        match list_dir(&self.dir, self.show_hidden) {
            Ok(items) => {
                self.items = items;
                self.browse_error = None;
            }
            Err(error) => {
                self.items.clear();
                self.browse_error = Some(error);
            }
        }
        let first = usize::from(self.items.len() > 1);
        self.browser
            .select((!self.items.is_empty()).then_some(first));
    }

    pub fn open_dir(&mut self, dir: PathBuf) {
        self.dir = dir;
        self.reload();
    }

    /// Go up one level, with the cursor on the folder just left.
    pub fn go_up(&mut self) {
        let Some(parent) = self.dir.parent().map(Path::to_path_buf) else {
            return;
        };
        let came_from = std::mem::replace(&mut self.dir, parent);
        self.reload();
        if let Some(at) = self.items.iter().position(|item| item.path == came_from) {
            self.browser.select(Some(at));
        }
    }

    pub fn current_item(&self) -> Option<&DirItem> {
        self.items.get(self.browser.selected()?)
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.known.contains(&canonical(path))
    }

    pub fn files(&self) -> Vec<PathBuf> {
        self.basket.iter().map(|entry| entry.path.clone()).collect()
    }

    pub fn total_size(&self) -> u64 {
        self.basket.iter().map(|entry| entry.size).sum()
    }

    /// Add files and the contents of folders (see [`scan_inputs`]).
    pub fn add_paths(&mut self, paths: &[PathBuf]) -> AddReport {
        let scan = scan_inputs(paths, self.recursive);
        let mut report = AddReport {
            problems: scan.problems,
            ..AddReport::default()
        };
        for path in scan.files {
            let key = canonical(&path);
            if !self.known.insert(key) {
                report.duplicates += 1;
                continue;
            }
            let size = std::fs::metadata(&path).map_or(0, |meta| meta.len());
            self.basket.push(FileEntry { path, size });
            report.added += 1;
        }
        if self.basket_cursor.selected().is_none() && !self.basket.is_empty() {
            self.basket_cursor.select(Some(0));
        }
        report
    }

    pub fn remove_current(&mut self) {
        let Some(at) = self.basket_cursor.selected() else {
            return;
        };
        if at < self.basket.len() {
            let removed = self.basket.remove(at);
            self.known.remove(&canonical(&removed.path));
        }
        self.basket_cursor
            .select((!self.basket.is_empty()).then(|| at.min(self.basket.len() - 1)));
    }

    pub fn clear(&mut self) {
        self.basket.clear();
        self.known.clear();
        self.basket_cursor.select(None);
    }

    /// Back to the browser after the basket was emptied from the keyboard.
    pub fn typing(&self) -> bool {
        self.pane == Pane::Path
    }
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

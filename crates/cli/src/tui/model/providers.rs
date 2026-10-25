//! The second step: choosing providers.

use std::collections::BTreeSet;
use std::sync::Arc;

use mul_tor_core::Provider;
use mul_tor_core::availability::Health;
use ratatui::widgets::ListState;

use crate::tui::input::TextInput;

/// What the last reachability probe found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probe {
    Unknown,
    Online,
    Offline(String),
}

pub struct ProviderRow {
    pub id: String,
    pub provider: Arc<dyn Provider>,
    /// `false` when it needs an account that is not set up.
    pub usable: bool,
    pub probe: Probe,
    /// Lower-case name and aliases, for the filter.
    search: String,
}

impl ProviderRow {
    pub fn new(provider: Arc<dyn Provider>, usable: bool) -> Self {
        let info = provider.info();
        let mut search = info.name.to_lowercase();
        for alias in info.aliases {
            search.push(' ');
            search.push_str(&alias.to_lowercase());
        }
        Self {
            id: info.id(),
            provider,
            usable,
            probe: Probe::Unknown,
            search,
        }
    }

    /// Would `--all` pick it? Not if it needs an account, is known to be
    /// broken, or just failed the reachability probe.
    pub fn in_bulk_choices(&self) -> bool {
        self.usable
            && self.provider.info().reliability.included_in_bulk_choices()
            && !matches!(self.probe, Probe::Offline(_))
    }
}

/// A preset, resolved against the providers that can be used right now.
#[derive(Clone, Debug)]
pub struct PresetChoice {
    pub stem: String,
    pub title: String,
    pub description: String,
    pub ids: Vec<String>,
    pub link_format: Option<String>,
    /// What was left out, and why.
    pub notes: Vec<String>,
}

pub struct ProvidersState {
    pub rows: Vec<ProviderRow>,
    pub selected: BTreeSet<String>,
    pub filter: TextInput,
    /// Typing into the filter.
    pub filtering: bool,
    pub list: ListState,
    pub presets: Vec<PresetChoice>,
    /// Index into `presets` of the one in use.
    pub preset: Option<usize>,
    /// The reachability probe has not answered yet.
    pub probing: bool,
}

impl ProvidersState {
    pub fn new(rows: Vec<ProviderRow>, presets: Vec<PresetChoice>) -> Self {
        let mut list = ListState::default();
        list.select((!rows.is_empty()).then_some(0));
        Self {
            rows,
            selected: BTreeSet::new(),
            filter: TextInput::default(),
            filtering: false,
            list,
            presets,
            preset: None,
            probing: false,
        }
    }

    /// Indices into `rows` of the rows matching the filter.
    pub fn visible(&self) -> Vec<usize> {
        let needle = self.filter.text().trim().to_lowercase();
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| needle.is_empty() || row.search.contains(&needle))
            .map(|(index, _)| index)
            .collect()
    }

    pub fn current(&self) -> Option<&ProviderRow> {
        let visible = self.visible();
        visible
            .get(self.list.selected()?)
            .map(|&index| &self.rows[index])
    }

    pub fn is_selected(&self, row: &ProviderRow) -> bool {
        self.selected.contains(&row.id)
    }

    /// Tick or untick the row under the cursor. `Err` says why it cannot be ticked.
    pub fn toggle_current(&mut self) -> Result<(), String> {
        let Some(row) = self.current() else {
            return Ok(());
        };
        if !row.usable {
            return Err(format!(
                "{} needs an account: add its credentials to the config file",
                row.provider.info().name
            ));
        }
        let id = row.id.clone();
        if !self.selected.remove(&id) {
            self.selected.insert(id);
        }
        Ok(())
    }

    /// Tick every visible row that bulk choices include; returns how many.
    pub fn select_visible(&mut self) -> usize {
        let ids: Vec<String> = self
            .visible()
            .into_iter()
            .map(|index| &self.rows[index])
            .filter(|row| row.in_bulk_choices())
            .map(|row| row.id.clone())
            .collect();
        let count = ids.len();
        self.selected.extend(ids);
        count
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// Use preset `index` (or none): its providers become the selection.
    pub fn apply_preset(&mut self, index: Option<usize>) {
        self.preset = index;
        self.selected = index
            .and_then(|i| self.presets.get(i))
            .map(|preset| preset.ids.iter().cloned().collect())
            .unwrap_or_default();
    }

    pub fn link_format(&self) -> Option<String> {
        self.presets.get(self.preset?)?.link_format.clone()
    }

    pub fn set_probe_results(&mut self, results: Vec<(String, Health)>) {
        for (id, health) in results {
            if let Some(row) = self.rows.iter_mut().find(|row| row.id == id) {
                row.probe = match health {
                    Health::Up => Probe::Online,
                    Health::Down(reason) => Probe::Offline(reason),
                };
            }
        }
        self.probing = false;
    }

    /// The ticked providers, in list order.
    pub fn selected_providers(&self) -> Vec<Arc<dyn Provider>> {
        self.rows
            .iter()
            .filter(|row| self.selected.contains(&row.id))
            .map(|row| row.provider.clone())
            .collect()
    }

    /// Cursor back to the top after the filter changed.
    pub fn filter_changed(&mut self) {
        let any = !self.visible().is_empty();
        self.list.select(any.then_some(0));
    }
}

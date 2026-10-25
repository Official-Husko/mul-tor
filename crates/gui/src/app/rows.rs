//! The providers shown in the list.

use std::sync::Arc;

use mul_tor_core::Provider;

use super::MulTorApp;
use crate::tasks;

/// A provider as displayed: its normalised id plus the uploader.
pub struct ProviderRow {
    pub id: String,
    pub provider: Arc<dyn Provider>,
}

impl MulTorApp {
    /// The providers shown in the list: everything not on the blacklist.
    pub fn rebuild_rows(&mut self) {
        self.rows = self
            .registry
            .iter()
            .filter(|provider| !self.config.is_blacklisted(provider.info()))
            .map(|provider| ProviderRow {
                id: provider.info().id(),
                provider: provider.clone(),
            })
            .collect();
        let known: Vec<String> = self.rows.iter().map(|r| r.id.clone()).collect();
        self.model.selected.retain(|id| known.contains(id));
    }

    pub fn lacks_credentials(&self, provider: &Arc<dyn Provider>) -> bool {
        self.config.lacks_credentials(provider.info())
    }

    pub(super) fn check_availability(&mut self) {
        self.model.checking = true;
        let providers = self
            .rows
            .iter()
            .map(|r| (r.id.clone(), r.provider.clone()))
            .collect();
        tasks::check_availability(
            self.rt.handle(),
            &self.bus,
            self.config.clone(),
            self.dirs.clone(),
            providers,
        );
    }
}

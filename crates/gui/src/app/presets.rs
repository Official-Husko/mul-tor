//! Applying a preset to the provider selection.

use std::sync::Arc;

use super::MulTorApp;
use crate::tasks::Level;

impl MulTorApp {
    pub(super) fn apply_preset(&mut self, stem: Option<String>) {
        let Some(stem) = stem else {
            self.model.preset = None;
            return;
        };
        let Some(preset) = self.presets.find(&stem) else {
            self.show_notice(Level::Error, format!("No preset called {stem}"));
            return;
        };
        let resolved = preset.resolve(&self.registry);
        let mut skipped = Vec::new();
        let mut broken = Vec::new();
        let mut ids = Vec::new();
        for provider in &resolved.providers {
            let Some(row) = self
                .rows
                .iter()
                .find(|r| Arc::ptr_eq(&r.provider, provider))
            else {
                continue; // blacklisted
            };
            if self.config.lacks_credentials(provider.info()) {
                skipped.push(provider.info().name);
            } else if !provider.info().reliability.included_in_bulk_choices() {
                broken.push(provider.info().name);
            } else {
                ids.push(row.id.clone());
            }
        }
        self.model.select_only(ids.iter().map(String::as_str));
        self.model.preset = Some(stem);

        let mut remarks = Vec::new();
        if !skipped.is_empty() {
            remarks.push(format!("needs credentials: {}", skipped.join(", ")));
        }
        if !broken.is_empty() {
            remarks.push(format!("known not to work: {}", broken.join(", ")));
        }
        if !resolved.unsupported.is_empty() {
            remarks.push(format!(
                "not supported yet: {}",
                resolved.unsupported.join(", ")
            ));
        }
        if !remarks.is_empty() {
            self.show_notice(
                Level::Info,
                format!("Preset applied; {}", remarks.join("; ")),
            );
        }
    }
}

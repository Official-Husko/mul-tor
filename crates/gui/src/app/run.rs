//! Starting an upload.

use std::sync::Arc;

use mul_tor_core::Provider;

use super::MulTorApp;
use crate::tasks::{self, RunRequest};

impl MulTorApp {
    pub(super) fn start(&mut self) {
        if !self.model.can_start() {
            return;
        }
        let providers: Vec<Arc<dyn Provider>> = self
            .rows
            .iter()
            .filter(|row| self.model.selected.contains(&row.id))
            .map(|row| row.provider.clone())
            .collect();
        let files = self.model.files.iter().map(|f| f.path.clone()).collect();
        let link_format = self
            .model
            .preset
            .as_deref()
            .and_then(|stem| self.presets.find(stem))
            .map(|preset| preset.link_format.clone())
            .filter(|format| !format.is_empty());

        self.model.begin_run(link_format.clone());
        self.cancel = Some(tasks::start_run(
            self.rt.handle(),
            &self.bus,
            RunRequest {
                files,
                providers,
                config: self.config.clone(),
                dirs: self.dirs.clone(),
                link_format,
                #[cfg(test)]
                url_rewrite: self.url_rewrite.clone(),
            },
        ));
    }
}

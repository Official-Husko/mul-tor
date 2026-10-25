//! The settings window's state, and loading and saving it.

use std::collections::BTreeMap;

use mul_tor_core::Config;

use super::MulTorApp;
use crate::tasks::Level;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    General,
    Accounts,
}

#[derive(Default)]
pub struct Settings {
    pub open: bool,
    pub tab: SettingsTab,
    /// Edited copy of the config; applied on save.
    pub draft: Config,
    /// Text being edited per `(provider id, credential key)`.
    pub buffers: BTreeMap<(String, String), String>,
    pub reveal_secrets: bool,
}

impl MulTorApp {
    pub fn open_settings(&mut self) {
        let mut buffers = BTreeMap::new();
        for row in &self.rows {
            let info = row.provider.info();
            let current = self.config.credentials_for(info);
            for field in info.credentials {
                buffers.insert(
                    (row.id.clone(), field.key.to_string()),
                    current.get(field.key).unwrap_or_default().to_string(),
                );
            }
        }
        self.settings = Settings {
            open: true,
            draft: self.config.clone(),
            buffers,
            ..Settings::default()
        };
    }

    pub(super) fn save_settings(&mut self) {
        let mut config = self.settings.draft.clone();
        for row in &self.rows {
            let info = row.provider.info();
            if info.credentials.is_empty() {
                continue;
            }
            let mut credentials = mul_tor_core::Credentials::default();
            for field in info.credentials {
                let value = self
                    .settings
                    .buffers
                    .get(&(row.id.clone(), field.key.to_string()))
                    .map(|v| v.trim().to_string())
                    .unwrap_or_default();
                if !value.is_empty() {
                    credentials.set(field.key, value);
                }
            }
            config.set_credentials(info, credentials);
        }
        config.parallel_uploads = config.parallel();
        match self.store.save(&config) {
            Ok(()) => {
                self.config = config;
                self.settings.open = false;
                self.rebuild_rows();
                self.show_notice(Level::Info, "Settings saved");
            }
            Err(error) => self.show_notice(
                Level::Error,
                format!("Could not save the settings: {error}"),
            ),
        }
    }
}

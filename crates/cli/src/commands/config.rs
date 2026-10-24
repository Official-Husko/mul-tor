//! `mul-tor config`: where the settings live, and what they are.

use anyhow::{Context as _, Result};

use crate::app::App;
use crate::args::ConfigAction;

pub fn config(app: &App, action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Path => println!("{}", app.store.path().display()),
        ConfigAction::Show => {
            let mut config = app.config.clone();
            for credentials in config.credentials.values_mut() {
                let masked: Vec<(String, String)> = credentials
                    .iter()
                    .map(|(key, value)| {
                        let shown = if value.trim().is_empty() {
                            ""
                        } else {
                            "********"
                        };
                        (key.to_string(), shown.to_string())
                    })
                    .collect();
                *credentials = masked.into_iter().collect();
            }
            print!(
                "{}",
                toml::to_string_pretty(&config).context("could not print the config")?
            );
        }
    }
    Ok(())
}

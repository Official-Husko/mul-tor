//! Working out which providers a request means.

use std::sync::Arc;

use anyhow::{Result, bail};
use mul_tor_core::Provider;

use crate::app::App;

/// What the user asked for on the command line.
#[derive(Default)]
pub struct Request<'a> {
    pub names: &'a [String],
    pub preset: Option<&'a str>,
    pub all: bool,
}

pub struct Selection {
    pub providers: Vec<Arc<dyn Provider>>,
    /// Template for the formatted link file, when a preset was used.
    pub link_format: Option<String>,
    /// Things worth telling the user (skipped providers and why).
    pub notes: Vec<String>,
}

/// `Ok(None)` means the request named nothing, so the caller should ask the
/// user (interactive) or complain (scripted).
///
/// - Named providers (`--provider`) are taken as given, even if their credentials are
///   missing, so the failure is explicit.
/// - Presets and `--all` skip blacklisted providers, providers whose credentials
///   are not configured, and providers recorded as broken, since those would
///   only fail. Naming a broken provider still works, with a warning.
/// - With nothing requested, a preset the config marks `auto_load` applies.
pub fn select(app: &App, request: &Request) -> Result<Option<Selection>> {
    let explicit = !request.names.is_empty();
    let preset_name = match request.preset {
        Some(name) => Some(name.to_string()),
        None if !explicit && !request.all => {
            let settings = &app.config.preset;
            (settings.auto_load && !settings.name.trim().is_empty()).then(|| settings.name.clone())
        }
        None => None,
    };
    if !explicit && !request.all && preset_name.is_none() {
        return Ok(None);
    }

    let mut selection = Selection {
        providers: Vec::new(),
        link_format: None,
        notes: Vec::new(),
    };
    let mut skipped_no_credentials: Vec<&str> = Vec::new();
    let mut skipped_broken: Vec<String> = Vec::new();

    let mut add = |selection: &mut Selection, provider: &Arc<dyn Provider>, skip_unusable: bool| {
        let info = provider.info();
        if selection.providers.iter().any(|s| Arc::ptr_eq(s, provider)) {
            return;
        }
        if app.config.is_blacklisted(info) {
            if !skip_unusable {
                selection
                    .notes
                    .push(format!("{} is on your blacklist; skipping it", info.name));
            }
            return;
        }
        if skip_unusable && app.lacks_credentials(info) {
            skipped_no_credentials.push(info.name);
            return;
        }
        if info.reliability.is_broken() {
            if skip_unusable {
                skipped_broken.push(format!("{} ({})", info.name, info.reliability));
                return;
            }
            selection.notes.push(format!(
                "{} is recorded as {}; trying it anyway",
                info.name, info.reliability
            ));
        }
        selection.providers.push(provider.clone());
    };

    if let Some(name) = preset_name {
        let Some(preset) = app.presets.find(&name) else {
            bail!("there is no preset called `{name}` (see `mul-tor presets`)");
        };
        let resolved = preset.resolve(&app.registry);
        if !resolved.unsupported.is_empty() {
            selection.notes.push(format!(
                "preset `{}` also lists providers Mul-Tor does not support: {}",
                preset.name,
                resolved.unsupported.join(", ")
            ));
        }
        for provider in &resolved.providers {
            add(&mut selection, provider, true);
        }
        if !preset.link_format.is_empty() {
            selection.link_format = Some(preset.link_format.clone());
        }
    }

    if request.all {
        for provider in app.registry.iter() {
            add(&mut selection, provider, true);
        }
    }

    if explicit {
        let (found, unknown) = app
            .registry
            .resolve(request.names.iter().map(String::as_str));
        if !unknown.is_empty() {
            bail!(
                "unknown provider{}: {} (see `mul-tor providers`)",
                if unknown.len() == 1 { "" } else { "s" },
                unknown.join(", ")
            );
        }
        for provider in &found {
            add(&mut selection, provider, false);
        }
    }

    if !skipped_no_credentials.is_empty() {
        selection.notes.push(format!(
            "skipped, no credentials configured: {}",
            skipped_no_credentials.join(", ")
        ));
    }
    if !skipped_broken.is_empty() {
        selection.notes.push(format!(
            "skipped, known not to work: {} (name one with --provider to try it anyway)",
            skipped_broken.join(", ")
        ));
    }
    if selection.providers.is_empty() {
        let mut message = String::from("no usable providers were selected");
        for note in &selection.notes {
            message.push_str(&format!("\n  - {note}"));
        }
        bail!(message);
    }
    Ok(Some(selection))
}

#[cfg(test)]
mod tests;

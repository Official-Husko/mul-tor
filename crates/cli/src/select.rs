//! Working out which sites a request means.

use std::sync::Arc;

use anyhow::{Result, bail};
use mul_tor_core::Site;

use crate::app::App;

/// What the user asked for on the command line.
#[derive(Default)]
pub struct Request<'a> {
    pub names: &'a [String],
    pub preset: Option<&'a str>,
    pub all: bool,
}

pub struct Selection {
    pub sites: Vec<Arc<dyn Site>>,
    /// Template for the formatted link file, when a preset was used.
    pub link_format: Option<String>,
    /// Things worth telling the user (skipped sites and why).
    pub notes: Vec<String>,
}

/// `Ok(None)` means the request named nothing, so the caller should ask the
/// user (interactive) or complain (scripted).
///
/// - Named sites (`--site`) are taken as given, even if their credentials are
///   missing, so the failure is explicit.
/// - Presets and `--all` skip blacklisted sites and sites whose credentials
///   are not configured, since those would only fail.
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
        sites: Vec::new(),
        link_format: None,
        notes: Vec::new(),
    };
    let mut skipped_no_credentials: Vec<&str> = Vec::new();

    let mut add = |selection: &mut Selection, site: &Arc<dyn Site>, skip_unusable: bool| {
        let info = site.info();
        if selection.sites.iter().any(|s| Arc::ptr_eq(s, site)) {
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
        selection.sites.push(site.clone());
    };

    if let Some(name) = preset_name {
        let Some(preset) = app.presets.find(&name) else {
            bail!("there is no preset called `{name}` (see `mul-tor presets`)");
        };
        let resolved = preset.resolve(&app.registry);
        if !resolved.unsupported.is_empty() {
            selection.notes.push(format!(
                "preset `{}` also lists sites Mul-Tor does not support: {}",
                preset.name,
                resolved.unsupported.join(", ")
            ));
        }
        for site in &resolved.sites {
            add(&mut selection, site, true);
        }
        if !preset.link_format.is_empty() {
            selection.link_format = Some(preset.link_format.clone());
        }
    }

    if request.all {
        for site in app.registry.iter() {
            add(&mut selection, site, true);
        }
    }

    if explicit {
        let (found, unknown) = app
            .registry
            .resolve(request.names.iter().map(String::as_str));
        if !unknown.is_empty() {
            bail!(
                "unknown site{}: {} (see `mul-tor sites`)",
                if unknown.len() == 1 { "" } else { "s" },
                unknown.join(", ")
            );
        }
        for site in &found {
            add(&mut selection, site, false);
        }
    }

    if !skipped_no_credentials.is_empty() {
        selection.notes.push(format!(
            "skipped, no credentials configured: {}",
            skipped_no_credentials.join(", ")
        ));
    }
    if selection.sites.is_empty() {
        let mut message = String::from("no usable sites were selected");
        for note in &selection.notes {
            message.push_str(&format!("\n  - {note}"));
        }
        bail!(message);
    }
    Ok(Some(selection))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_app;
    use mul_tor_core::{Config, Credentials};

    fn names(selection: &Selection) -> Vec<&'static str> {
        selection.sites.iter().map(|s| s.info().name).collect()
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn nothing_requested_selects_nothing() {
        let app = test_app(Config::default());
        assert!(select(&app, &Request::default()).unwrap().is_none());
    }

    #[test]
    fn named_sites_are_resolved_loosely_in_order_without_duplicates() {
        let app = test_app(Config::default());
        let wanted = strings(&["oshi", "Transfer_sh", "OSHI"]);
        let selection = select(
            &app,
            &Request {
                names: &wanted,
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(names(&selection), ["Oshi", "Transfer.sh"]);
    }

    #[test]
    fn unknown_names_are_an_error_that_lists_them() {
        let app = test_app(Config::default());
        let wanted = strings(&["oshi", "nope", "alsonope"]);
        let err = select(
            &app,
            &Request {
                names: &wanted,
                ..Default::default()
            },
        )
        .err()
        .unwrap()
        .to_string();
        assert!(err.contains("nope, alsonope"), "{err}");
    }

    #[test]
    fn named_sites_keep_missing_credentials_so_the_failure_is_explicit() {
        let app = test_app(Config::default());
        let wanted = strings(&["pixeldrain"]);
        let selection = select(
            &app,
            &Request {
                names: &wanted,
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(names(&selection), ["Pixeldrain"]);
    }

    #[test]
    fn presets_skip_sites_that_lack_credentials_and_say_so() {
        let app = test_app(Config::default());
        let selection = select(
            &app,
            &Request {
                preset: Some("f95"),
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        let picked = names(&selection);
        assert!(picked.contains(&"Gofile") && picked.contains(&"Catbox"));
        assert!(!picked.contains(&"Pixeldrain") && !picked.contains(&"Mixdrop"));
        assert!(
            selection
                .notes
                .iter()
                .any(|n| n.contains("Pixeldrain") && n.contains("Mixdrop"))
        );
        assert_eq!(
            selection.link_format.as_deref(),
            Some("[URL='{file_url}']{site_name}[/URL]")
        );
    }

    #[test]
    fn configured_credentials_make_a_keyed_site_usable() {
        let mut config = Config::default();
        let mut creds = Credentials::default();
        creds.set("apiKey", "k");
        config.credentials.insert("Pixeldrain".into(), creds);
        let app = test_app(config);
        let selection = select(
            &app,
            &Request {
                preset: Some("f95"),
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        assert!(names(&selection).contains(&"Pixeldrain"));
    }

    #[test]
    fn all_excludes_blacklisted_and_credentialless_sites() {
        let config = Config {
            blacklist: strings(&["oshi"]),
            ..Config::default()
        };
        let app = test_app(config);
        let selection = select(
            &app,
            &Request {
                all: true,
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        let picked = names(&selection);
        assert!(!picked.contains(&"Oshi"));
        assert!(!picked.contains(&"BowFile"));
        assert!(picked.contains(&"Gofile"));
        assert_eq!(picked.len(), 37 - 1 - 5);
    }

    #[test]
    fn a_named_blacklisted_site_is_skipped_with_a_note() {
        let config = Config {
            blacklist: strings(&["oshi"]),
            ..Config::default()
        };
        let app = test_app(config);
        let wanted = strings(&["oshi", "gofile"]);
        let selection = select(
            &app,
            &Request {
                names: &wanted,
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(names(&selection), ["Gofile"]);
        assert!(selection.notes.iter().any(|n| n.contains("blacklist")));
    }

    #[test]
    fn auto_load_preset_applies_only_when_nothing_else_was_requested() {
        let mut config = Config::default();
        config.preset.auto_load = true;
        config.preset.name = "f95.json".into();
        let app = test_app(config);

        let auto = select(&app, &Request::default()).unwrap().unwrap();
        assert!(names(&auto).contains(&"Catbox"));

        let wanted = strings(&["oshi"]);
        let explicit = select(
            &app,
            &Request {
                names: &wanted,
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(names(&explicit), ["Oshi"]);
    }

    #[test]
    fn unknown_presets_are_an_error() {
        let app = test_app(Config::default());
        let err = select(
            &app,
            &Request {
                preset: Some("nope"),
                ..Default::default()
            },
        )
        .err()
        .unwrap()
        .to_string();
        assert!(err.contains("no preset called `nope`"), "{err}");
    }

    #[test]
    fn preset_and_named_sites_combine() {
        let app = test_app(Config::default());
        let wanted = strings(&["keep"]);
        let selection = select(
            &app,
            &Request {
                names: &wanted,
                preset: Some("f95"),
                all: false,
            },
        )
        .unwrap()
        .unwrap();
        assert!(names(&selection).contains(&"Keep") && names(&selection).contains(&"Gofile"));
    }
}

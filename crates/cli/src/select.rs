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
mod tests {
    use super::*;
    use crate::app::test_app;
    use mul_tor_core::{Config, Credentials};

    fn names(selection: &Selection) -> Vec<&'static str> {
        selection.providers.iter().map(|s| s.info().name).collect()
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
    fn named_providers_are_resolved_loosely_in_order_without_duplicates() {
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
    fn named_providers_keep_missing_credentials_so_the_failure_is_explicit() {
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
    fn presets_skip_providers_that_lack_credentials_and_say_so() {
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
    fn configured_credentials_make_a_keyed_provider_usable() {
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
    fn all_excludes_blacklisted_and_credentialless_providers() {
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
    fn a_named_blacklisted_provider_is_skipped_with_a_note() {
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
    fn preset_and_named_providers_combine() {
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

    mod broken_providers {
        use std::sync::Arc;

        use async_trait::async_trait;
        use mul_tor_core::{
            Provider, ProviderInfo, Registry, UploadContext, UploadError, UploadedFile,
        };

        use super::*;
        use crate::app::test_app_with;

        struct Stub(ProviderInfo);

        #[async_trait]
        impl Provider for Stub {
            fn info(&self) -> &ProviderInfo {
                &self.0
            }

            async fn upload(&self, _: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
                unreachable!("selection never uploads")
            }
        }

        fn app() -> App {
            let stub = |info| -> Arc<dyn Provider> { Arc::new(Stub(info)) };
            test_app_with(
                Config::default(),
                Registry::new(vec![
                    stub(ProviderInfo::new("Fine", "https://f", "https://f")),
                    stub(ProviderInfo::new("Shaky", "https://s", "https://s").degraded("slow")),
                    stub(ProviderInfo::new("Dead", "https://d", "https://d").broken("shut down")),
                ]),
            )
        }

        #[test]
        fn all_skips_broken_providers_and_says_so() {
            let selection = select(
                &app(),
                &Request {
                    all: true,
                    ..Default::default()
                },
            )
            .unwrap()
            .unwrap();
            assert_eq!(names(&selection), ["Fine", "Shaky"]);
            assert!(
                selection
                    .notes
                    .iter()
                    .any(|n| n.contains("Dead") && n.contains("shut down")),
                "{:?}",
                selection.notes
            );
        }

        #[test]
        fn naming_a_broken_provider_still_tries_it_with_a_warning() {
            let wanted = strings(&["dead"]);
            let selection = select(
                &app(),
                &Request {
                    names: &wanted,
                    ..Default::default()
                },
            )
            .unwrap()
            .unwrap();
            assert_eq!(names(&selection), ["Dead"]);
            assert!(selection.notes[0].contains("trying it anyway"));
        }

        #[test]
        fn a_degraded_provider_is_not_held_back() {
            let wanted = strings(&["shaky"]);
            let selection = select(
                &app(),
                &Request {
                    names: &wanted,
                    ..Default::default()
                },
            )
            .unwrap()
            .unwrap();
            assert_eq!(names(&selection), ["Shaky"]);
            assert!(selection.notes.is_empty());
        }
    }
}

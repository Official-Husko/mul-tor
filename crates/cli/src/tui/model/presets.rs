//! Resolving the presets against what can be used right now.

use super::providers::PresetChoice;
use crate::app::App;
use crate::selection::{Request, select};

/// Every preset that still leaves at least one usable provider, and the
/// index of the one the config asks to load automatically.
pub fn build(app: &App) -> (Vec<PresetChoice>, Option<usize>) {
    let auto = {
        let settings = &app.config.preset;
        (settings.auto_load && !settings.name.trim().is_empty())
            .then(|| app.presets.find(&settings.name))
            .flatten()
    };

    let mut choices = Vec::new();
    let mut auto_index = None;
    for (stem, preset) in app.presets.iter() {
        let request = Request {
            preset: Some(stem),
            ..Request::default()
        };
        let Ok(Some(selection)) = select(app, &request) else {
            continue; // nothing usable in it
        };
        if auto.is_some_and(|wanted| std::ptr::eq(wanted, preset)) {
            auto_index = Some(choices.len());
        }
        choices.push(PresetChoice {
            stem: stem.to_string(),
            title: if preset.name.is_empty() {
                stem.to_string()
            } else {
                preset.name.clone()
            },
            description: preset.description.clone(),
            ids: selection.providers.iter().map(|p| p.info().id()).collect(),
            link_format: selection.link_format,
            notes: selection.notes,
        });
    }
    (choices, auto_index)
}

#[cfg(test)]
mod tests {
    use mul_tor_core::Config;

    use super::*;
    use crate::app::test_app;

    #[test]
    fn bundled_presets_resolve_to_usable_providers() {
        let (choices, auto) = build(&test_app(Config::default()));
        let f95 = choices.iter().find(|c| c.stem == "f95").expect("f95");
        assert!(f95.ids.iter().any(|id| id == "gofile"));
        assert!(
            !f95.ids.iter().any(|id| id == "pixeldrain"),
            "needs an account"
        );
        assert!(f95.link_format.is_some());
        assert!(f95.notes.iter().any(|note| note.contains("no credentials")));
        assert_eq!(auto, None);
    }

    #[test]
    fn the_configured_preset_is_found_for_automatic_loading() {
        let mut config = Config::default();
        config.preset.auto_load = true;
        config.preset.name = "f95.json".into();
        let (choices, auto) = build(&test_app(config));
        assert_eq!(choices[auto.unwrap()].stem, "f95");
    }
}

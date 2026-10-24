//! Presets: named lists of sites (plus a link template) for a target forum or
//! community, so one choice selects everything that place accepts.

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::site::{Registry, Site, normalize_name};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preset {
    pub identifier: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub version: String,
    #[serde(rename = "site-url")]
    pub site_url: String,
    #[serde(rename = "source-url")]
    pub source_url: String,
    #[serde(rename = "repo-url")]
    pub repo_url: String,
    /// Site names, matched loosely against the registry.
    pub sites: Vec<String>,
    /// Template for the formatted link file; see [`crate::linklog`].
    #[serde(rename = "link-format")]
    pub link_format: String,
}

/// A preset matched against the sites this build supports.
#[derive(Clone)]
pub struct ResolvedPreset {
    pub sites: Vec<Arc<dyn Site>>,
    /// Names the preset lists that Mul-Tor has no uploader for.
    pub unsupported: Vec<String>,
}

impl Preset {
    pub fn parse(text: &str, origin: &str) -> Result<Preset> {
        serde_json::from_str(text).map_err(|e| Error::Preset {
            name: origin.to_string(),
            message: e.to_string(),
        })
    }

    pub fn resolve(&self, registry: &Registry) -> ResolvedPreset {
        let (sites, unsupported) = registry.resolve(self.sites.iter().map(String::as_str));
        ResolvedPreset { sites, unsupported }
    }

    /// Does `query` (file name, identifier or title) refer to this preset?
    fn matches(&self, stem: &str, query: &str) -> bool {
        let query = normalize_name(query.trim_end_matches(".json").trim_end_matches(".JSON"));
        !query.is_empty()
            && [stem, self.identifier.as_str(), self.name.as_str()]
                .iter()
                .any(|candidate| normalize_name(candidate) == query)
    }
}

/// The presets compiled into the program (the same files as `presets/` in the repo).
const BUILTIN: [(&str, &str); 2] = [
    ("f95", include_str!("../../../presets/f95.json")),
    ("cs-rin-ru", include_str!("../../../presets/cs-rin-ru.json")),
];

/// Built-in presets plus the user's own.
#[derive(Clone, Debug, Default)]
pub struct PresetLibrary {
    /// `(file stem, preset)`. A user file replaces a built-in of the same stem.
    entries: Vec<(String, Preset)>,
    /// User files that could not be read, with the reason.
    pub problems: Vec<(String, String)>,
}

impl PresetLibrary {
    pub fn builtin() -> Self {
        let entries = BUILTIN
            .iter()
            .map(|(stem, text)| {
                let preset = Preset::parse(text, stem).expect("built-in presets are valid");
                (stem.to_string(), preset)
            })
            .collect();
        Self {
            entries,
            problems: Vec::new(),
        }
    }

    /// Built-ins plus every `*.json` in `dir` (a missing directory is fine).
    pub fn load(dir: &Path) -> Self {
        let mut library = Self::builtin();
        let Ok(read) = std::fs::read_dir(dir) else {
            return library;
        };
        let mut files: Vec<_> = read
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            })
            .collect();
        files.sort();

        for path in files {
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let parsed = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| Preset::parse(&text, &stem).map_err(|e| e.to_string()));
            match parsed {
                Ok(preset) => {
                    library.entries.retain(|(existing, _)| existing != &stem);
                    library.entries.push((stem, preset));
                }
                Err(reason) => library.problems.push((path.display().to_string(), reason)),
            }
        }
        library
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Preset)> {
        self.entries
            .iter()
            .map(|(stem, preset)| (stem.as_str(), preset))
    }

    pub fn find(&self, query: &str) -> Option<&Preset> {
        self.entries
            .iter()
            .find(|(stem, preset)| preset.matches(stem, query))
            .map(|(_, preset)| preset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_presets_load() {
        let library = PresetLibrary::builtin();
        let f95 = library.find("f95").unwrap();
        assert_eq!(f95.identifier, "husko.f95");
        assert_eq!(f95.link_format, "[URL='{file_url}']{site_name}[/URL]");
        assert!(library.find("cs-rin-ru").is_some());
    }

    #[test]
    fn presets_are_found_by_file_name_identifier_or_title() {
        let library = PresetLibrary::builtin();
        for query in ["f95", "F95", "f95.json", "husko.f95", "  "] {
            assert_eq!(
                library.find(query).is_some(),
                query.trim() != "",
                "{query:?}"
            );
        }
        assert!(library.find("CS.Rin.Ru").is_some());
        assert!(library.find("husko.csrinru").is_some());
        assert!(library.find("nope").is_none());
    }

    #[test]
    fn f95_resolves_fully_with_the_current_registry() {
        let resolved = PresetLibrary::builtin()
            .find("f95")
            .unwrap()
            .resolve(&Registry::builtin());
        assert!(
            resolved.unsupported.is_empty(),
            "{:?}",
            resolved.unsupported
        );
        let names: Vec<_> = resolved.sites.iter().map(|s| s.info().name).collect();
        assert!(names.contains(&"Pixeldrain") && names.contains(&"Download.gg"));
    }

    #[test]
    fn unsupported_names_are_reported_not_dropped_silently() {
        let resolved = PresetLibrary::builtin()
            .find("cs-rin-ru")
            .unwrap()
            .resolve(&Registry::builtin());
        assert!(resolved.unsupported.iter().any(|n| n == "MEGA"));
        // Differently-cased legacy spellings still resolve.
        let names: Vec<_> = resolved.sites.iter().map(|s| s.info().name).collect();
        assert!(names.contains(&"BowFile") && names.contains(&"UserCloud"));
    }

    #[test]
    fn user_presets_override_builtins_and_bad_files_are_reported() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("f95.json"),
            r#"{"identifier": "me.f95", "name": "Mine", "sites": ["Oshi"]}"#,
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("extra.json"),
            r#"{"name": "Extra", "sites": []}"#,
        )
        .unwrap();
        std::fs::write(tmp.path().join("broken.json"), "{ nope").unwrap();
        std::fs::write(tmp.path().join("notes.txt"), "ignored").unwrap();

        let library = PresetLibrary::load(tmp.path());
        assert_eq!(library.find("f95").unwrap().identifier, "me.f95");
        assert!(library.find("extra").is_some());
        assert!(
            library.find("cs-rin-ru").is_some(),
            "other built-ins survive"
        );
        assert_eq!(library.problems.len(), 1);
        assert!(library.problems[0].0.ends_with("broken.json"));
    }

    #[test]
    fn a_missing_presets_directory_is_not_an_error() {
        let library = PresetLibrary::load(Path::new("/definitely/not/here"));
        assert!(library.problems.is_empty());
        assert!(library.find("f95").is_some());
    }
}

//! Reading and writing `config.toml`.

use std::path::{Path, PathBuf};

use super::model::Config;
use crate::error::{Error, Result};
use crate::paths::AppDirs;

/// How the config was obtained at startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigOrigin {
    /// Read from an existing `config.toml`.
    File,
    /// `config.toml` did not exist; a documented default was written.
    Created,
    /// `config.toml` did not exist; the Python version's `config.json` was imported.
    ImportedLegacy,
}

/// Loads and saves `config.toml`.
#[derive(Clone, Debug)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(dirs: &AppDirs) -> Self {
        Self {
            path: dirs.config_file(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the config, creating it if needed.
    ///
    /// If there is no `config.toml` yet, a `config.json` in `legacy_dir` (the
    /// Python version kept it next to the program) is imported; the old file
    /// is left untouched.
    pub fn load_or_create(&self, legacy_dir: Option<&Path>) -> Result<(Config, ConfigOrigin)> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => return Ok((Config::parse(&text, &self.path)?, ConfigOrigin::File)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(format!("reading {}", self.path.display()), e)),
        }

        if let Some(dir) = legacy_dir {
            let legacy_path = dir.join("config.json");
            if let Ok(text) = std::fs::read_to_string(&legacy_path) {
                let config = Config::from_legacy_json(&text, &legacy_path)?;
                self.save(&config)?;
                return Ok((config, ConfigOrigin::ImportedLegacy));
            }
        }

        self.write(Config::template())?;
        Ok((Config::default(), ConfigOrigin::Created))
    }

    /// Write `config`. Comments in a hand-edited file are not preserved.
    pub fn save(&self, config: &Config) -> Result<()> {
        let body = toml::to_string_pretty(config)
            .map_err(|e| Error::Other(format!("could not serialise the config: {e}")))?;
        self.write(&format!(
            "# Mul-Tor settings. Saved by the program, so comments are not kept;\n\
             # see the documentation for what each option does.\n\n{body}"
        ))
    }

    fn write(&self, contents: &str) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::io(format!("creating {}", parent.display()), e))?;
        }
        std::fs::write(&self.path, contents)
            .map_err(|e| Error::io(format!("writing {}", self.path.display()), e))?;
        restrict_permissions(&self.path);
        Ok(())
    }
}

/// The file holds API keys and passwords; keep other users out where the
/// platform supports it.
fn restrict_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::legacy::tests::LEGACY;

    #[test]
    fn store_creates_a_documented_default_on_first_run() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(&AppDirs::in_dir(tmp.path()));
        let (config, origin) = store.load_or_create(None).unwrap();
        assert_eq!(origin, ConfigOrigin::Created);
        assert_eq!(config, Config::default());
        assert!(std::fs::read_to_string(store.path()).unwrap().contains('#'));

        let (_, origin) = store.load_or_create(None).unwrap();
        assert_eq!(origin, ConfigOrigin::File);
    }

    #[test]
    fn store_imports_a_legacy_config_once_and_leaves_it_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let legacy_dir = tmp.path().join("old");
        std::fs::create_dir(&legacy_dir).unwrap();
        std::fs::write(legacy_dir.join("config.json"), LEGACY).unwrap();

        let store = ConfigStore::new(&AppDirs::in_dir(tmp.path().join("new")));
        let (config, origin) = store.load_or_create(Some(&legacy_dir)).unwrap();
        assert_eq!(origin, ConfigOrigin::ImportedLegacy);
        assert_eq!(config.credentials["Pixeldrain"].get("apiKey"), Some("pd"));
        assert!(legacy_dir.join("config.json").exists());

        // Second start reads the new file, not the legacy one.
        let (again, origin) = store.load_or_create(Some(&legacy_dir)).unwrap();
        assert_eq!(origin, ConfigOrigin::File);
        assert_eq!(again, config);
    }

    #[test]
    fn a_broken_config_is_an_error_not_silently_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(&AppDirs::in_dir(tmp.path()));
        std::fs::write(store.path(), "this is = not [valid").unwrap();
        assert!(store.load_or_create(None).is_err());
        assert_eq!(
            std::fs::read_to_string(store.path()).unwrap(),
            "this is = not [valid"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_config_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(&AppDirs::in_dir(tmp.path()));
        store.load_or_create(None).unwrap();
        let mode = std::fs::metadata(store.path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "mode is {mode:o}");
    }
}

//! Where Mul-Tor keeps its files.
//!
//! The Python version dropped everything next to wherever it was started.
//! This version uses the platform's config/data/cache directories, unless a
//! home directory is given explicitly (`--home` / `MUL_TOR_HOME`), in which
//! case everything lives in that one folder (handy for a portable install).

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct AppDirs {
    config: PathBuf,
    data: PathBuf,
    cache: PathBuf,
}

impl AppDirs {
    pub fn discover(home_override: Option<&Path>) -> Result<Self> {
        if let Some(home) = home_override {
            return Ok(Self::in_dir(home));
        }
        let dirs = ProjectDirs::from("", "", "mul-tor").ok_or_else(|| {
            Error::Other(
                "could not determine a home directory; set MUL_TOR_HOME to choose one".into(),
            )
        })?;
        Ok(Self {
            config: dirs.config_dir().to_path_buf(),
            data: dirs.data_dir().to_path_buf(),
            cache: dirs.cache_dir().to_path_buf(),
        })
    }

    /// Everything in one directory.
    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            config: dir.clone(),
            data: dir.clone(),
            cache: dir,
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config.join("config.toml")
    }

    /// User presets; built-in ones are compiled in.
    pub fn presets_dir(&self) -> PathBuf {
        self.config.join("presets")
    }

    pub fn user_agents_file(&self) -> PathBuf {
        self.cache.join("user_agents.json")
    }

    /// Daily log files; see [`crate::logging`].
    pub fn log_dir(&self) -> PathBuf {
        self.data.join("logs")
    }

    /// Default place for the link files in the GUI. (The CLI defaults to the
    /// current directory, like the Python version.)
    pub fn links_dir(&self) -> PathBuf {
        self.data.join("links")
    }

    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    pub fn create_all(&self) -> Result<()> {
        for dir in [&self.config, &self.data, &self.cache] {
            std::fs::create_dir_all(dir)
                .map_err(|e| Error::io(format!("creating {}", dir.display()), e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_home_puts_everything_in_one_folder() {
        let dirs = AppDirs::discover(Some(Path::new("/tmp/mt"))).unwrap();
        assert_eq!(dirs.config_file(), Path::new("/tmp/mt/config.toml"));
        assert_eq!(dirs.presets_dir(), Path::new("/tmp/mt/presets"));
        assert_eq!(
            dirs.user_agents_file(),
            Path::new("/tmp/mt/user_agents.json")
        );
        assert_eq!(dirs.log_dir(), Path::new("/tmp/mt/logs"));
    }

    #[test]
    fn create_all_makes_the_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = AppDirs::in_dir(tmp.path().join("a/b"));
        dirs.create_all().unwrap();
        assert!(tmp.path().join("a/b").is_dir());
    }
}

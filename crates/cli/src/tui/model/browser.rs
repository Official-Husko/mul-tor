//! Listing one folder for the file browser.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    /// The `..` entry that goes up one level.
    pub is_parent: bool,
}

/// The folder's contents: `..` first, then folders, then files, each sorted
/// by name without regard to case. Unreadable entries are skipped.
pub fn list_dir(dir: &Path, show_hidden: bool) -> Result<Vec<DirItem>, String> {
    let entries = std::fs::read_dir(dir).map_err(|error| error.to_string())?;
    let mut items: Vec<DirItem> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !show_hidden && name.starts_with('.') {
                return None;
            }
            // `metadata` follows symlinks, so a link to a folder opens like one.
            let meta = std::fs::metadata(entry.path()).ok()?;
            Some(DirItem {
                name,
                path: entry.path(),
                is_dir: meta.is_dir(),
                size: if meta.is_dir() { 0 } else { meta.len() },
                is_parent: false,
            })
        })
        .collect();
    items.sort_by_cached_key(|item| (!item.is_dir, item.name.to_lowercase()));

    if let Some(parent) = dir.parent() {
        items.insert(
            0,
            DirItem {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
                size: 0,
                is_parent: true,
            },
        );
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(items: &[DirItem]) -> Vec<&str> {
        items.iter().map(|item| item.name.as_str()).collect()
    }

    #[test]
    fn parent_first_then_folders_then_files_ignoring_case() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("Zeta")).unwrap();
        std::fs::create_dir(tmp.path().join("alpha")).unwrap();
        std::fs::write(tmp.path().join("b.txt"), "1").unwrap();
        std::fs::write(tmp.path().join("A.txt"), "22").unwrap();

        let items = list_dir(tmp.path(), false).unwrap();
        assert_eq!(names(&items), ["..", "alpha", "Zeta", "A.txt", "b.txt"]);
        assert!(items[0].is_parent && items[1].is_dir);
        assert_eq!(items[3].size, 2);
    }

    #[test]
    fn hidden_entries_appear_only_when_asked_for() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".secret"), "x").unwrap();
        std::fs::write(tmp.path().join("shown"), "x").unwrap();
        assert!(!names(&list_dir(tmp.path(), false).unwrap()).contains(&".secret"));
        assert!(names(&list_dir(tmp.path(), true).unwrap()).contains(&".secret"));
    }

    #[test]
    fn a_missing_folder_is_an_error_not_a_panic() {
        assert!(list_dir(Path::new("/definitely/not/here"), false).is_err());
    }
}

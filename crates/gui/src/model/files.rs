//! The files chosen for upload.

use std::path::PathBuf;

use mul_tor_core::files::scan_inputs;

use super::Model;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

impl Model {
    /// Add files (folders contribute the files directly inside them).
    /// Returns the inputs that could not be used.
    pub fn add_paths(&mut self, paths: &[PathBuf]) -> Vec<(PathBuf, String)> {
        let scan = scan_inputs(paths, false);
        for path in scan.files {
            if self.files.iter().any(|f| f.path == path) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            self.files.push(FileEntry {
                path,
                name,
                size: meta.len(),
            });
        }
        scan.problems
    }

    pub fn remove_file(&mut self, index: usize) {
        if index < self.files.len() {
            self.files.remove(index);
        }
    }

    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

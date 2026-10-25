//! Turning the paths a user hands over (files and folders) into the list of
//! files to upload.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Deepest folder nesting followed when scanning recursively.
const MAX_DEPTH: usize = 32;

#[derive(Debug, Default)]
pub struct InputScan {
    /// Files to upload, in a stable order, without duplicates.
    pub files: Vec<PathBuf>,
    /// Inputs that could not be used, with the reason.
    pub problems: Vec<(PathBuf, String)>,
}

/// Expand `inputs`. Files are taken as they are; a folder contributes the files
/// directly inside it (sorted by name), or everything below it if `recursive`.
pub fn scan_inputs(inputs: &[PathBuf], recursive: bool) -> InputScan {
    let mut scan = InputScan::default();
    let mut seen = HashSet::new();
    for input in inputs {
        match std::fs::metadata(input) {
            Ok(meta) if meta.is_file() => push(&mut scan, &mut seen, input.clone()),
            Ok(meta) if meta.is_dir() => walk(input, recursive, 0, &mut scan, &mut seen),
            Ok(_) => scan
                .problems
                .push((input.clone(), "not a regular file or folder".into())),
            Err(e) => scan.problems.push((input.clone(), e.to_string())),
        }
    }
    scan
}

fn push(scan: &mut InputScan, seen: &mut HashSet<PathBuf>, path: PathBuf) {
    let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    if seen.insert(key) {
        scan.files.push(path);
    }
}

fn walk(
    dir: &Path,
    recursive: bool,
    depth: usize,
    scan: &mut InputScan,
    seen: &mut HashSet<PathBuf>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            scan.problems.push((dir.to_path_buf(), e.to_string()));
            return;
        }
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        // `file_type` does not follow symlinks, which keeps symlinked folders
        // from causing loops; symlinked files are still picked up below.
        let Ok(kind) = std::fs::symlink_metadata(&path).map(|m| m.file_type()) else {
            continue;
        };
        if kind.is_dir() {
            if recursive && depth < MAX_DEPTH {
                walk(&path, recursive, depth + 1, scan, seen);
            }
        } else if path.is_file() {
            push(scan, seen, path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("b.txt"), "b").unwrap();
        std::fs::write(tmp.path().join("a.txt"), "a").unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/c.txt"), "c").unwrap();
        tmp
    }

    fn names(scan: &InputScan) -> Vec<String> {
        scan.files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn folders_contribute_their_direct_files_sorted() {
        let tmp = tree();
        let scan = scan_inputs(&[tmp.path().to_path_buf()], false);
        assert_eq!(names(&scan), ["a.txt", "b.txt"]);
        assert!(scan.problems.is_empty());
    }

    #[test]
    fn recursive_scan_descends() {
        let tmp = tree();
        let scan = scan_inputs(&[tmp.path().to_path_buf()], true);
        assert_eq!(names(&scan), ["a.txt", "b.txt", "c.txt"]);
    }

    #[test]
    fn duplicates_are_removed_even_via_different_spellings() {
        let tmp = tree();
        let a = tmp.path().join("a.txt");
        let a_again = tmp.path().join("sub/../a.txt");
        let scan = scan_inputs(&[a, a_again, tmp.path().to_path_buf()], false);
        assert_eq!(names(&scan), ["a.txt", "b.txt"]);
    }

    #[test]
    fn missing_inputs_are_reported_and_do_not_stop_the_rest() {
        let tmp = tree();
        let scan = scan_inputs(
            &[tmp.path().join("nope.txt"), tmp.path().join("a.txt")],
            false,
        );
        assert_eq!(names(&scan), ["a.txt"]);
        assert_eq!(scan.problems.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_loops_do_not_hang_the_scan() {
        let tmp = tree();
        std::os::unix::fs::symlink(tmp.path(), tmp.path().join("sub/loop")).unwrap();
        let scan = scan_inputs(&[tmp.path().to_path_buf()], true);
        assert_eq!(names(&scan), ["a.txt", "b.txt", "c.txt"]);
    }
}

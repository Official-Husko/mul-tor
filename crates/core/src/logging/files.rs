//! Log files: a new one each day, the oldest ones removed, readable back.
//!
//! Files are named `mul-tor.YYYY-MM-DD.log`. The date is UTC (that is how the
//! rotation works); the timestamps inside are local time with their offset.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::{NonBlocking, NonBlockingBuilder, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

const PREFIX: &str = "mul-tor";
const SUFFIX: &str = "log";

/// How many daily files are kept.
pub const KEEP_FILES: usize = 14;

/// Lines waiting for the writer thread before a logging call has to wait.
const QUEUE_LINES: usize = 8192;

/// How much of the end of a file [`tail`] looks at.
const TAIL_WINDOW: u64 = 512 * 1024;

/// A background writer into `dir`, created if needed.
///
/// The returned guard flushes the queue when dropped, so hold on to it until
/// the program ends.
pub(super) fn open(dir: &Path) -> io::Result<(NonBlocking, WorkerGuard)> {
    create_private_dir(dir)?;
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(PREFIX)
        .filename_suffix(SUFFIX)
        .max_log_files(KEEP_FILES)
        .build(dir)
        .map_err(io::Error::other)?;
    Ok(NonBlockingBuilder::default()
        .lossy(false)
        .buffered_lines_limit(QUEUE_LINES)
        .thread_name("mul-tor-log")
        .finish(appender))
}

/// Logs can quote what a provider answered, so keep other users out.
fn create_private_dir(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Every log file in `dir`, newest first.
pub fn list_logs(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with(&format!("{PREFIX}.")) && name.ends_with(&format!(".{SUFFIX}"))
                })
        })
        .collect();
    // The date is in the name, so name order is time order.
    files.sort_by(|a, b| b.cmp(a));
    files
}

pub fn latest_log(dir: &Path) -> Option<PathBuf> {
    list_logs(dir).into_iter().next()
}

/// The last `lines` lines of `path`, reading only the end of a large file.
pub fn tail(path: &Path, lines: usize) -> io::Result<Vec<String>> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let start = length.saturating_sub(TAIL_WINDOW);
    file.seek(SeekFrom::Start(start))?;

    let mut bytes = Vec::with_capacity((length - start) as usize);
    file.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);

    let mut all: Vec<&str> = text.lines().collect();
    if start > 0 && !all.is_empty() {
        all.remove(0); // began in the middle of a line
    }
    let skip = all.len().saturating_sub(lines);
    Ok(all[skip..].iter().map(|line| line.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn logs_are_listed_newest_first_and_strangers_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "mul-tor.2026-10-22.log", "");
        write(tmp.path(), "mul-tor.2026-10-24.log", "");
        write(tmp.path(), "mul-tor.2026-10-23.log", "");
        write(tmp.path(), "notes.txt", "");
        write(tmp.path(), "runtime.log", "");

        let names: Vec<_> = list_logs(tmp.path())
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "mul-tor.2026-10-24.log",
                "mul-tor.2026-10-23.log",
                "mul-tor.2026-10-22.log"
            ]
        );
        assert!(latest_log(&tmp.path().join("missing")).is_none());
    }

    #[test]
    fn tail_returns_the_last_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write(tmp.path(), "a.log", "one\ntwo\nthree\nfour\n");
        assert_eq!(tail(&path, 2).unwrap(), ["three", "four"]);
        assert_eq!(tail(&path, 99).unwrap().len(), 4);
        assert!(tail(&tmp.path().join("missing.log"), 1).is_err());
    }

    #[test]
    fn tail_of_a_huge_file_drops_the_cut_line() {
        let tmp = tempfile::tempdir().unwrap();
        let line = format!("{}\n", "x".repeat(99));
        let path = write(tmp.path(), "big.log", &line.repeat(20_000)); // ~2 MB
        let last = tail(&path, 5).unwrap();
        assert_eq!(last.len(), 5);
        assert!(last.iter().all(|l| l.len() == 99));
    }

    #[cfg(unix)]
    #[test]
    fn the_log_folder_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("logs");
        let (_writer, _guard) = open(&dir).unwrap();
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "mode is {mode:o}");
    }
}

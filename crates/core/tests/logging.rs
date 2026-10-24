//! `logging::init` end to end: file, in-memory buffer, levels and panics.
//!
//! The logger is process-wide, so this file holds a single test.

use mul_tor_core::logging::{self, LogBuffer, LogConfig};
use tracing::Level;

#[test]
fn init_writes_files_fills_the_buffer_and_captures_panics() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("logs");
    let buffer = LogBuffer::new(100);

    let guard = logging::init(
        LogConfig::new()
            .directory(&dir)
            .level(Level::INFO)
            .buffer(buffer.clone()),
    );
    assert!(guard.problems().is_empty(), "{:?}", guard.problems());
    assert_eq!(guard.directory(), Some(dir.as_path()));

    {
        let span =
            tracing::info_span!(target: "mul_tor_core::test", "job", id = 1, provider = "Oshi");
        let _entered = span.enter();
        tracing::info!(target: "mul_tor_core::test", link_host = "oshi.at", "upload finished");
        tracing::debug!(target: "mul_tor_core::test", "below the level, kept out of both outputs");
    }
    let crash = std::thread::spawn(|| panic!("boom"));
    assert!(crash.join().is_err());

    // Dropping the guard flushes the queue to disk.
    drop(guard);

    let file = logging::latest_log(&dir).expect("a log file was created");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("upload finished"), "{text}");
    assert!(text.contains("link_host=\"oshi.at\""), "{text}");
    assert!(text.contains("job{id=1 provider=\"Oshi\"}"), "{text}");
    assert!(text.contains("the program panicked"), "{text}");
    assert!(text.contains("boom"), "{text}");
    assert!(!text.contains("below the level"), "{text}");

    let records = buffer.snapshot();
    let finished = records
        .iter()
        .find(|r| r.message == "upload finished")
        .expect("the buffer saw the event");
    assert_eq!(finished.spans, "job{id=1 provider=Oshi}");
    assert!(
        records
            .iter()
            .all(|r| r.message != "below the level, kept out of both outputs")
    );
    assert!(records.iter().any(|r| r.message == "the program panicked"));

    // A second init is refused politely.
    let second = logging::init(LogConfig::new());
    assert_eq!(second.problems().len(), 1);
}

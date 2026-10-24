//! The two text outputs: the log file and the terminal.

use std::io::IsTerminal;

use tracing::Level;
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::{self, MakeWriter};
use tracing_subscriber::registry::Registry;

use super::filter;
use super::time::{ConsoleTime, FileTime};

pub(super) type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

/// Full records: local timestamp with offset, level, span context, module,
/// source line, message and fields. Spans also log how long they took when
/// they close. With `json`, the same information as one JSON object per line.
pub(super) fn file<W>(writer: W, json: bool, level: Level) -> BoxedLayer
where
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    let layer = fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_timer(FileTime)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE);
    if json {
        layer
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_filter(filter::build(level))
            .boxed()
    } else {
        layer.with_filter(filter::build(level)).boxed()
    }
}

/// Short lines on stderr, in colour when that is a terminal.
pub(super) fn console(level: Level) -> BoxedLayer {
    let colour = std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(colour)
        .with_timer(ConsoleTime)
        .with_target(true)
        .with_filter(filter::build(level))
        .boxed()
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::{Arc, Mutex};

    use regex::Regex;
    use tracing::subscriber::with_default;
    use tracing_subscriber::prelude::*;

    use super::*;

    /// A writer that keeps everything in memory.
    #[derive(Clone, Default)]
    struct Memory(Arc<Mutex<Vec<u8>>>);

    impl io::Write for Memory {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Memory {
        type Writer = Memory;

        fn make_writer(&'a self) -> Memory {
            self.clone()
        }
    }

    impl Memory {
        fn text(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    fn emit() {
        let span = tracing::info_span!("job", id = 7, provider = "Gofile");
        let _entered = span.enter();
        tracing::info!(elapsed_ms = 12, "upload finished");
        tracing::debug!("too chatty for the default level");
    }

    #[test]
    fn a_line_has_timestamp_level_span_module_source_and_fields() {
        let out = Memory::default();
        with_default(
            tracing_subscriber::registry().with(file(out.clone(), false, Level::INFO)),
            emit,
        );
        let text = out.text();
        let line = text
            .lines()
            .find(|l| l.contains("upload finished"))
            .unwrap();

        let shape = Regex::new(
            r#"^\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d{3} [+-]\d\d:\d\d +INFO job\{id=7 provider="Gofile"\}: mul_tor_core::logging::layers::tests: crates/core/src/logging/layers\.rs:\d+: upload finished elapsed_ms=12$"#,
        )
        .unwrap();
        assert!(shape.is_match(line), "{line}");
        assert!(!text.contains("too chatty"), "debug is below the level");
    }

    #[test]
    fn closing_a_span_logs_how_long_it_took() {
        let out = Memory::default();
        with_default(
            tracing_subscriber::registry().with(file(out.clone(), false, Level::INFO)),
            emit,
        );
        assert!(out.text().contains("close time.busy="), "{}", out.text());
    }

    #[test]
    fn json_lines_carry_the_same_information() {
        let out = Memory::default();
        with_default(
            tracing_subscriber::registry().with(file(out.clone(), true, Level::INFO)),
            emit,
        );
        let text = out.text();
        let line = text
            .lines()
            .find(|l| l.contains("upload finished"))
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(value["level"], "INFO");
        assert_eq!(value["fields"]["message"], "upload finished");
        assert_eq!(value["fields"]["elapsed_ms"], 12);
        assert_eq!(value["span"]["name"], "job");
        assert!(value["timestamp"].as_str().unwrap().contains(':'));
        assert!(value["filename"].as_str().unwrap().ends_with("layers.rs"));
    }
}

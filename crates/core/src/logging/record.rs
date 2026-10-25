//! One log record, as kept in memory for a UI.

use chrono::{DateTime, Local};
use tracing::Level;

use super::time::FILE_FORMAT;

#[derive(Clone, Debug)]
pub struct LogRecord {
    /// Grows by one per record; lets a UI ask for "what is new".
    pub seq: u64,
    pub time: DateTime<Local>,
    pub level: Level,
    /// The module that logged it, e.g. `mul_tor_core::engine::job`.
    pub target: String,
    /// `file.rs:41`, when known.
    pub location: String,
    /// The enclosing spans, e.g. `job{id=3 provider=Pixeldrain}`.
    pub spans: String,
    pub message: String,
    pub fields: Vec<(String, String)>,
}

impl LogRecord {
    pub fn timestamp(&self) -> String {
        self.time.format(FILE_FORMAT).to_string()
    }

    /// `key=value key=value`, in the order they were logged.
    pub fn fields_text(&self) -> String {
        self.fields
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The record on one line, laid out like the log file.
    pub fn line(&self) -> String {
        let mut line = format!("{} {:>5} ", self.timestamp(), self.level);
        if !self.spans.is_empty() {
            line.push_str(&self.spans);
            line.push_str(": ");
        }
        line.push_str(&self.target);
        line.push_str(": ");
        if !self.location.is_empty() {
            line.push_str(&self.location);
            line.push_str(": ");
        }
        line.push_str(&self.message);
        if !self.fields.is_empty() {
            line.push(' ');
            line.push_str(&self.fields_text());
        }
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_has_every_part_in_order() {
        let record = LogRecord {
            seq: 1,
            time: Local::now(),
            level: Level::WARN,
            target: "mul_tor_core::engine::job".into(),
            location: "job.rs:50".into(),
            spans: "job{id=3}".into(),
            message: "upload failed".into(),
            fields: vec![("provider".into(), "Oshi".into())],
        };
        let line = record.line();
        let after_time = line.split_once(' ').unwrap().1;
        assert!(
            after_time.contains(
                "WARN job{id=3}: mul_tor_core::engine::job: job.rs:50: upload failed provider=Oshi"
            ),
            "{line}"
        );
    }
}

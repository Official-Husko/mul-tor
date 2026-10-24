//! Reading the fields of an event or span into plain strings.

use std::fmt::Debug;

use tracing::field::{Field, Visit};

/// Collects `message` separately from the other fields.
#[derive(Default)]
pub(super) struct FieldCollector {
    pub message: String,
    pub fields: Vec<(String, String)>,
}

impl Visit for FieldCollector {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields
                .push((field.name().to_string(), format!("{value:?}")));
        }
    }
}

impl FieldCollector {
    /// `id=3 provider=Pixeldrain`, in the order the fields were given.
    pub(super) fn span_text(&self) -> String {
        self.fields
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

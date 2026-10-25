//! The `tracing` layer that turns events into [`LogRecord`]s.

use chrono::Local;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

use super::buffer::LogBuffer;
use super::record::LogRecord;
use super::visitor::FieldCollector;

/// Fields of a span, rendered once when it is created.
struct SpanFields(String);

pub struct BufferLayer {
    buffer: LogBuffer,
}

impl BufferLayer {
    pub(super) fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

impl<S> Layer<S> for BufferLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut collector = FieldCollector::default();
        attrs.record(&mut collector);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut()
                .insert(SpanFields(collector.span_text()));
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let mut collector = FieldCollector::default();
        values.record(&mut collector);
        let Some(span) = ctx.span(id) else { return };
        if let Some(fields) = span.extensions_mut().get_mut::<SpanFields>() {
            if !fields.0.is_empty() {
                fields.0.push(' ');
            }
            fields.0.push_str(&collector.span_text());
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut collector = FieldCollector::default();
        event.record(&mut collector);

        let spans = ctx
            .event_scope(event)
            .map(|scope| {
                scope
                    .from_root()
                    .map(|span| {
                        let extensions = span.extensions();
                        match extensions.get::<SpanFields>() {
                            Some(fields) if !fields.0.is_empty() => {
                                format!("{}{{{}}}", span.name(), fields.0)
                            }
                            _ => span.name().to_string(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(":")
            })
            .unwrap_or_default();

        // A record from another crate's `log` calls arrives wrapped; this
        // unwraps it to its own target, file and line.
        let normalized = event.normalized_metadata();
        let metadata = normalized.as_ref().unwrap_or_else(|| event.metadata());
        self.buffer.push(LogRecord {
            seq: 0,
            time: Local::now(),
            level: *metadata.level(),
            target: metadata.target().to_string(),
            location: match (metadata.file(), metadata.line()) {
                (Some(file), Some(line)) => format!("{file}:{line}"),
                _ => String::new(),
            },
            spans,
            message: collector.message,
            fields: collector.fields,
        });
    }
}

#[cfg(test)]
mod tests {
    use tracing::subscriber::with_default;
    use tracing_subscriber::prelude::*;

    use super::*;

    fn capture(buffer: &LogBuffer, run: impl FnOnce()) {
        let subscriber = tracing_subscriber::registry().with(buffer.layer());
        with_default(subscriber, run);
    }

    #[test]
    fn events_keep_their_message_fields_and_source() {
        let buffer = LogBuffer::new(10);
        capture(&buffer, || {
            tracing::warn!(provider = "Oshi", attempts = 3, "gave up");
        });
        let record = &buffer.snapshot()[0];
        assert_eq!(record.level, tracing::Level::WARN);
        assert_eq!(record.message, "gave up");
        assert_eq!(
            record.fields,
            [
                ("provider".to_string(), "Oshi".to_string()),
                ("attempts".to_string(), "3".to_string())
            ]
        );
        assert!(record.target.ends_with("logging::capture::tests"));
        assert!(
            record.location.contains("capture.rs:"),
            "{}",
            record.location
        );
    }

    #[test]
    fn spans_are_listed_outermost_first_with_their_fields() {
        let buffer = LogBuffer::new(10);
        capture(&buffer, || {
            let run = tracing::info_span!("run", files = 2);
            let _run = run.enter();
            let job = tracing::info_span!("job", id = 7, provider = "Gofile");
            let _job = job.enter();
            tracing::info!("started");
        });
        assert_eq!(
            buffer.snapshot()[0].spans,
            "run{files=2}:job{id=7 provider=Gofile}"
        );
    }

    #[test]
    fn the_oldest_records_make_room() {
        let buffer = LogBuffer::new(3);
        capture(&buffer, || {
            for n in 1..=5 {
                tracing::info!(n, "tick");
            }
        });
        let seqs: Vec<_> = buffer.snapshot().iter().map(|r| r.seq).collect();
        assert_eq!(seqs, [3, 4, 5]);
        assert_eq!(buffer.last_seq(), 5);
    }

    #[test]
    fn records_from_many_threads_stay_in_sequence_order() {
        let buffer = LogBuffer::new(10_000);
        let subscriber = std::sync::Arc::new(tracing_subscriber::registry().with(buffer.layer()));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let subscriber = subscriber.clone();
                std::thread::spawn(move || {
                    tracing::subscriber::with_default(subscriber, || {
                        for n in 0..500 {
                            tracing::info!(n, "tick");
                        }
                    });
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let seqs: Vec<_> = buffer.snapshot().iter().map(|r| r.seq).collect();
        assert_eq!(seqs.len(), 4000);
        assert!(
            seqs.windows(2).all(|pair| pair[0] < pair[1]),
            "out of order"
        );
    }

    #[test]
    fn tail_returns_the_newest_records_oldest_first() {
        let buffer = LogBuffer::new(10);
        capture(&buffer, || {
            for n in 1..=6 {
                tracing::info!(n, "tick");
            }
        });
        let seqs: Vec<_> = buffer.tail(3).iter().map(|r| r.seq).collect();
        assert_eq!(seqs, [4, 5, 6]);
        assert_eq!(buffer.tail(99).len(), 6);
        assert!(buffer.tail(0).is_empty());
    }

    #[test]
    fn since_returns_only_what_is_new() {
        let buffer = LogBuffer::new(10);
        capture(&buffer, || {
            tracing::info!("one");
            tracing::info!("two");
        });
        let seen = buffer.last_seq();
        capture(&buffer, || tracing::info!("three"));
        let new = buffer.since(seen);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].message, "three");
        assert_eq!(buffer.since(0).len(), 3);
        buffer.clear();
        assert!(buffer.is_empty());
    }
}

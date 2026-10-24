//! The most recent records, kept in memory so a UI can show them live.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use super::capture::BufferLayer;
use super::record::LogRecord;

/// A bounded, shareable list of the latest [`LogRecord`]s. Cloning is cheap
/// and every clone sees the same records.
#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Inner>,
}

struct Inner {
    capacity: usize,
    next_seq: AtomicU64,
    records: Mutex<VecDeque<LogRecord>>,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            inner: Arc::new(Inner {
                capacity,
                next_seq: AtomicU64::new(1),
                records: Mutex::new(VecDeque::with_capacity(capacity.min(1024))),
            }),
        }
    }

    /// Everything currently held, oldest first.
    pub fn snapshot(&self) -> Vec<LogRecord> {
        self.records().iter().cloned().collect()
    }

    /// Records newer than `seq` (use `0` for all), oldest first.
    pub fn since(&self, seq: u64) -> Vec<LogRecord> {
        self.records()
            .iter()
            .filter(|record| record.seq > seq)
            .cloned()
            .collect()
    }

    /// Sequence number of the newest record (`0` if none yet).
    pub fn last_seq(&self) -> u64 {
        self.inner.next_seq.load(Ordering::Relaxed) - 1
    }

    pub fn len(&self) -> usize {
        self.records().len()
    }

    pub fn is_empty(&self) -> bool {
        self.records().is_empty()
    }

    pub fn clear(&self) {
        self.records().clear();
    }

    /// The `tracing` layer that feeds this buffer.
    pub fn layer(&self) -> BufferLayer {
        BufferLayer::new(self.clone())
    }

    fn records(&self) -> std::sync::MutexGuard<'_, VecDeque<LogRecord>> {
        self.inner
            .records
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn push(&self, mut record: LogRecord) {
        record.seq = self.inner.next_seq.fetch_add(1, Ordering::Relaxed);
        let mut records = self.records();
        if records.len() == self.inner.capacity {
            records.pop_front();
        }
        records.push_back(record);
    }
}

impl std::fmt::Debug for LogBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogBuffer")
            .field("records", &self.len())
            .finish()
    }
}

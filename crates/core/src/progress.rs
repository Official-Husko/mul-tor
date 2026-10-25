use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

/// Shared, lock-light progress state for one upload.
///
/// Uploaders push into it (bytes handed to the HTTP stack, a short status line
/// for multi-step flows); the engine samples it on a timer and turns the
/// samples into [`Event::Progress`](crate::engine::Event). Cloning shares the
/// underlying state.
#[derive(Clone, Debug, Default)]
pub struct Progress(Arc<Inner>);

#[derive(Debug, Default)]
struct Inner {
    sent: AtomicU64,
    total: AtomicU64,
    message: Mutex<String>,
}

/// A point-in-time copy of a [`Progress`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgressSnapshot {
    pub sent: u64,
    pub total: u64,
    pub message: String,
}

impl Progress {
    pub fn new(total: u64) -> Self {
        let progress = Self::default();
        progress.0.total.store(total, Ordering::Relaxed);
        progress
    }

    pub fn add(&self, bytes: u64) {
        self.0.sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn sent(&self) -> u64 {
        self.0.sent.load(Ordering::Relaxed)
    }

    pub fn total(&self) -> u64 {
        self.0.total.load(Ordering::Relaxed)
    }

    /// Short status for multi-step uploads ("Requesting upload server", "Chunk 3/12").
    pub fn set_message(&self, message: impl Into<String>) {
        *self.lock() = message.into();
    }

    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            sent: self.sent(),
            total: self.total(),
            message: self.lock().clone(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, String> {
        // A poisoned lock only means another thread panicked mid-write of a
        // status string; the string itself is still usable.
        self.0
            .message
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_state() {
        let a = Progress::new(100);
        let b = a.clone();
        b.add(40);
        b.set_message("halfway");
        let snap = a.snapshot();
        assert_eq!(snap.sent, 40);
        assert_eq!(snap.total, 100);
        assert_eq!(snap.message, "halfway");
    }
}

//! Runs uploads: every file to every selected provider, reporting as it goes.
//!
//! The engine is the one piece the CLI and the GUI share. It knows nothing
//! about terminals or windows; it reports through an [`EventSink`] callback.

mod event;
mod job;
mod plan;
mod schedule;

use std::collections::BTreeMap;
use std::sync::Arc;

pub use event::{Event, EventSink, JobId, JobOutcome};

use crate::http::HttpFactory;
use crate::linklog::LinkLog;
use crate::provider::Credentials;

#[derive(Clone)]
pub struct Engine {
    http: HttpFactory,
    credentials: Arc<BTreeMap<String, Credentials>>,
    parallel: usize,
    link_log: Option<LinkLog>,
}

impl Engine {
    pub fn new(http: HttpFactory) -> Self {
        Self {
            http,
            credentials: Arc::default(),
            parallel: 1,
            link_log: None,
        }
    }

    /// Credentials by provider id (see [`crate::Config::credentials_by_id`]).
    pub fn with_credentials(mut self, credentials: BTreeMap<String, Credentials>) -> Self {
        self.credentials = Arc::new(credentials);
        self
    }

    /// How many uploads may run at once (at least 1).
    pub fn with_parallelism(mut self, parallel: usize) -> Self {
        self.parallel = parallel.max(1);
        self
    }

    /// Record every successful upload.
    pub fn with_link_log(mut self, log: Option<LinkLog>) -> Self {
        self.link_log = log;
        self
    }
}

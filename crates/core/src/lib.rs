//! Core of Mul-Tor: upload a file to many hosting providers at once.
//!
//! This crate has no user interface. The `mul-tor` CLI and `mul-tor-gui`
//! binaries are thin front-ends over it.

pub mod engine;
pub mod error;
pub mod files;
pub mod logging;
mod net;
pub mod progress;
pub mod provider;
pub mod providers;
pub mod setup;
pub mod size;
mod storage;
pub mod upload;

// The network and storage modules are grouped on disk but live at the crate
// root, so `mul_tor_core::http`, `::config` and friends stay as they are.
pub(crate) use net::reply;
pub use net::{availability, http, proxies, update, useragents};
pub use storage::{config, linklog, paths, preset};

pub use config::{Config, ConfigOrigin, ConfigStore};
pub use engine::{Engine, Event, EventSink, JobId, JobOutcome};
pub use error::{Error, Result, UploadError};
pub use http::{Http, HttpFactory, ProxyMode};
pub use linklog::{LinkEntry, LinkLog};
pub use paths::AppDirs;
pub use preset::{Preset, PresetLibrary};
pub use progress::{Progress, ProgressSnapshot};
pub use provider::{
    Auth, CredentialField, Credentials, Provider, ProviderInfo, Registry, normalize_name,
};
pub use size::{SizeLimit, format_bytes};
pub use upload::{LocalFile, UploadContext, UploadedFile};

//! Core of Mul-Tor: upload a file to many hosting sites at once.
//!
//! This crate has no user interface. The `mul-tor` CLI and `mul-tor-gui`
//! binaries are thin front-ends over it.

pub mod availability;
pub mod config;
pub mod engine;
pub mod error;
pub mod files;
pub mod http;
pub mod linklog;
pub mod paths;
pub mod preset;
pub mod progress;
pub mod proxies;
mod reply;
pub mod setup;
pub mod site;
pub mod sites;
pub mod size;
pub mod update;
pub mod upload;
pub mod useragents;

pub use config::{Config, ConfigOrigin, ConfigStore};
pub use engine::{Engine, Event, EventSink, JobId, JobOutcome};
pub use error::{Error, Result, UploadError};
pub use http::{Http, HttpFactory, ProxyMode};
pub use linklog::{LinkEntry, LinkLog};
pub use paths::AppDirs;
pub use preset::{Preset, PresetLibrary};
pub use progress::{Progress, ProgressSnapshot};
pub use site::{Auth, CredentialField, Credentials, Registry, Site, SiteInfo, normalize_name};
pub use size::{SizeLimit, format_bytes};
pub use upload::{LocalFile, UploadContext, UploadedFile};

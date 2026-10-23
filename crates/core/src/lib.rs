//! Core of Mul-Tor: upload a file to many hosting sites at once.
//!
//! This crate has no user interface. The `mul-tor` CLI and `mul-tor-gui`
//! binaries are thin front-ends over it.

pub mod error;
pub mod http;
pub mod progress;
mod reply;
pub mod site;
pub mod sites;
pub mod size;
pub mod upload;

pub use error::{Error, Result, UploadError};
pub use http::{Http, HttpFactory, ProxyMode};
pub use progress::{Progress, ProgressSnapshot};
pub use site::{Auth, CredentialField, Credentials, Registry, Site, SiteInfo, normalize_name};
pub use size::{SizeLimit, format_bytes};
pub use upload::{LocalFile, UploadContext, UploadedFile};

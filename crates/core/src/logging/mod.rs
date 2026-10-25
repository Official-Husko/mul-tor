//! Logging: what is recorded, where it goes, and how it reads.
//!
//! Every record carries a local timestamp (milliseconds and UTC offset), the
//! level, the enclosing spans (`job{id=3 provider="Pixeldrain"}`), the module
//! that logged it and its source line. Records go to
//!
//! * a daily file, kept for [`KEEP_FILES`] days, always;
//! * stderr, when the user asks with `-v` (never in a full-screen UI);
//! * a [`LogBuffer`], so a UI can show them live.
//!
//! Secrets stay out: [`Credentials`](crate::Credentials) print only key names,
//! and a URL is logged through [`redact_url`] so no query string leaks.
//! `MUL_TOR_LOG` overrides the level, e.g. `MUL_TOR_LOG=debug`.

mod buffer;
mod capture;
mod files;
mod filter;
mod init;
mod layers;
mod panic;
mod record;
mod redact;
mod time;
mod visitor;

pub use buffer::LogBuffer;
pub use files::{KEEP_FILES, latest_log, list_logs, tail};
pub use filter::{ENV_VAR, console_level, parse_level};
pub use init::{LogConfig, LogGuard, init};
pub use record::LogRecord;
pub use redact::redact_url;

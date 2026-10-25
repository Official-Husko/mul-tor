//! Test harness: an in-process HTTP server that records every request, plus
//! helpers to run an uploader against it.
//!
//! Uploaders talk to their real `https://host/path` URLs. [`Mock::rewrite`]
//! maps those onto `http://127.0.0.1:PORT/host/path`, so a handler can tell
//! providers apart by `host` and nothing ever leaves the machine.

// Each test binary uses a different part of the kit.
#![allow(dead_code, unused_imports)]

mod multipart;
mod recorded;
mod reply;
mod run;
mod server;
mod socket;

pub use multipart::{FormPart, field, parse_multipart, text_field};
pub use recorded::Recorded;
pub use reply::Reply;
pub use run::{Run, RunConfig, builtin, creds, run, run_default};
pub use server::Mock;
pub use socket::Frame;

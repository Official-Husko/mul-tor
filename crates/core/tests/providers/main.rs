//! Wire-level tests: each provider against an in-process mock server, checking
//! exactly what it sends and how it reads the reply.
//!
//! Each test pins what the Python version sent (field names, headers, paths)
//! and what it extracted from the reply, so a refactor cannot silently change
//! the protocol.

#[path = "../common/mod.rs"]
mod common;

mod chunked;
mod fileio;
mod filesfm;
mod form_post;
mod forms;
mod gofile;
mod jqfu;
mod laravel;
mod onefichier;
mod rapidgator;
mod single_request;
mod support;
mod two_step;
mod xfs;

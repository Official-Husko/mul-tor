//! End-to-end tests: run the real `mul-tor` binary against a local mock server.
//!
//! Needs the `test-hooks` feature, which lets `MUL_TOR_TEST_BASE_URL` redirect
//! every provider to localhost. Without the hook these tests would talk to the real
//! providers, so the whole file is compiled out unless it is enabled:
//!
//!     cargo test -p mul-tor --features test-hooks
#![cfg(feature = "test-hooks")]

mod commands;
mod common;
mod selecting;
mod uploading;

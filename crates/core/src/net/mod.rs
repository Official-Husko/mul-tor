//! Everything that talks to the network apart from the providers themselves:
//! the HTTP client, reply parsing, proxy and user-agent lists, availability
//! probes and update checks.

pub mod availability;
pub mod http;
pub mod proxies;
pub(crate) mod reply;
pub mod update;
pub mod useragents;

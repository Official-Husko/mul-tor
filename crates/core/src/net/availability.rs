//! Is a provider reachable right now?
//!
//! The Python version pinged each provider one after another (up to five seconds
//! each, so a bad day meant minutes at startup) and only accepted HTTP 200.
//! Here the probes run concurrently and any answer that proves the host is up
//! counts, including the 403/429 some providers give to non-browsers.

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use reqwest::StatusCode;

use crate::http::HttpFactory;
use crate::provider::Provider;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const CONCURRENCY: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Health {
    Up,
    Down(String),
}

impl Health {
    pub fn is_up(&self) -> bool {
        matches!(self, Health::Up)
    }
}

#[derive(Clone)]
pub struct ProviderStatus {
    pub provider: Arc<dyn Provider>,
    pub health: Health,
}

/// Probe every provider; results are in the same order as `providers`.
pub async fn check(factory: &HttpFactory, providers: &[Arc<dyn Provider>]) -> Vec<ProviderStatus> {
    // Each probe owns its inputs. Building the futures up front, rather than
    // inside a `Stream::map` closure, keeps this future spawnable on a
    // multi-threaded runtime (rustc cannot prove the closure form `Send`).
    let probes: Vec<_> = providers
        .iter()
        .cloned()
        .map(|provider| {
            let factory = factory.clone();
            async move {
                let health = probe(&factory, &provider).await;
                match &health {
                    Health::Down(reason) => tracing::warn!(
                        provider = provider.info().name,
                        %reason,
                        "availability check failed"
                    ),
                    other => tracing::debug!(
                        provider = provider.info().name,
                        health = ?other,
                        "availability checked"
                    ),
                }
                ProviderStatus { provider, health }
            }
        })
        .collect();
    futures::stream::iter(probes)
        .buffered(CONCURRENCY)
        .collect()
        .await
}

async fn probe(factory: &HttpFactory, provider: &Arc<dyn Provider>) -> Health {
    let http = match factory.client() {
        Ok(http) => http,
        Err(error) => return Health::Down(format!("could not set up a connection: {error}")),
    };
    match http
        .get(provider.info().probe_url)
        .timeout(PROBE_TIMEOUT)
        .send()
        .await
    {
        Ok(response) => judge(response.status()),
        Err(error) if error.is_timeout() => Health::Down("timed out".into()),
        Err(error) if error.is_connect() => Health::Down("connection failed".into()),
        Err(error) => Health::Down(error.without_url().to_string()),
    }
}

/// Any answer that is not a server error shows the host is alive. 401/403/405/429
/// are what providers tend to answer a bare GET with; they still accept uploads.
fn judge(status: StatusCode) -> Health {
    let alive = status.is_success()
        || status.is_redirection()
        || matches!(status.as_u16(), 401 | 403 | 405 | 429);
    if alive {
        Health::Up
    } else {
        Health::Down(format!("HTTP {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn judging_status_codes() {
        for code in [200, 204, 301, 302, 401, 403, 405, 429] {
            assert!(judge(StatusCode::from_u16(code).unwrap()).is_up(), "{code}");
        }
        for code in [400, 404, 410, 500, 502, 503, 521] {
            assert!(
                !judge(StatusCode::from_u16(code).unwrap()).is_up(),
                "{code}"
            );
        }
    }
}

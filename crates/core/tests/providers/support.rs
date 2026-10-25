//! Helpers shared by the provider test files.

use crate::common::*;

pub fn only_request(mock: &Mock) -> Recorded {
    let requests = mock.requests();
    assert_eq!(requests.len(), 1, "expected one request, got {requests:#?}");
    requests.into_iter().next().unwrap()
}

/// 25 bytes: three chunks of at most 10.
pub const CONTENT_25: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXY";

pub fn config_25() -> RunConfig {
    RunConfig {
        name: "data.bin".into(),
        content: CONTENT_25.to_vec(),
        ..Default::default()
    }
}

pub fn chunks_of(mock: &Mock, host: &str, path: &str) -> Vec<Recorded> {
    mock.requests()
        .into_iter()
        .filter(|r| r.host == host && r.path == path && r.method == "POST")
        .collect()
}

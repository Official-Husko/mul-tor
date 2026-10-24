//! What a Lufi instance tells its web client, read from the two files that do.

use std::sync::LazyLock;

use regex::Regex;
use reqwest::Url;

/// `var ws_url  = 'wss://host/upload';` in `partial/index.js`.
static WS_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"var\s+ws_url\s*=\s*'(wss://[^']+)'"#).expect("pattern is valid")
});

static DELAY_SELECT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<select[^>]*id="delete-day"[^>]*>(.*?)</select>"#).expect("pattern is valid")
});

static OPTION_VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<option[^>]*value="(\d+)""#).expect("pattern is valid"));

/// Where the WebSocket lives, as an `https` address (what the HTTP client
/// upgrades), if it is on `host`. The file goes there, so anything else is refused.
pub(super) fn socket_address(index_js: &str, host: &str) -> Option<String> {
    let found = WS_URL.captures(index_js)?.get(1)?.as_str();
    let mut url = Url::parse(found).ok()?;
    let on_host = url.host_str() == Some(host)
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none();
    if !on_host || url.set_scheme("https").is_err() {
        return None;
    }
    Some(url.into())
}

/// The longest time, in days, the instance keeps a file: `0` (no limit) if the
/// page offers it, else the largest choice.
pub(super) fn longest_delay(page: &str) -> Option<u32> {
    let options = DELAY_SELECT.captures(page)?.get(1)?.as_str();
    let days: Vec<u32> = OPTION_VALUE
        .captures_iter(options)
        .filter_map(|c| c[1].parse().ok())
        .collect();
    if days.contains(&0) {
        Some(0)
    } else {
        days.into_iter().max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX_JS: &str =
        "var ws_url  = 'wss://upload.adminforge.de/upload';\nvar maxSize = 2147483648;";

    #[test]
    fn the_socket_must_be_on_the_instances_own_host() {
        assert_eq!(
            socket_address(INDEX_JS, "upload.adminforge.de").as_deref(),
            Some("https://upload.adminforge.de/upload")
        );
        assert_eq!(socket_address(INDEX_JS, "evil.example"), None);
        for bad in [
            "var ws_url = 'wss://evil.example/upload';",
            "var ws_url = 'wss://user@upload.adminforge.de/upload';",
            "var ws_url = 'wss://upload.adminforge.de:8443/upload';",
            "var ws_url = 'ws://upload.adminforge.de/upload';",
            "nothing here",
        ] {
            assert_eq!(socket_address(bad, "upload.adminforge.de"), None, "{bad}");
        }
    }

    #[test]
    fn the_longest_delay_is_the_largest_choice_or_no_limit() {
        let page = r#"<select id="delete-day"><option value="1" >24 hours</option>
            <option value="7" selected="selected">7 days</option>
            <option value="30" >30 days</option></select>"#;
        assert_eq!(longest_delay(page), Some(30));
        let unlimited = r#"<select id="delete-day"><option value="0">No limit</option>
            <option value="365">1 year</option></select>"#;
        assert_eq!(longest_delay(unlimited), Some(0));
        assert_eq!(longest_delay("<select id=\"other\"></select>"), None);
        assert_eq!(longest_delay(r#"<select id="delete-day"></select>"#), None);
    }
}

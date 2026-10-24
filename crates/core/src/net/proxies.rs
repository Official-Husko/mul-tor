//! A pool of public HTTP proxies for `use_proxies`.

use std::time::Duration;

use futures::future::join_all;

use crate::http::Http;

/// Plain-text `ip:port` lists, one entry per line.
pub const SOURCES: [&str; 6] = [
    "https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/http.txt",
    "https://raw.githubusercontent.com/ShiftyTR/Proxy-List/master/http.txt",
    "https://raw.githubusercontent.com/mmpx12/proxy-list/master/http.txt",
    "https://raw.githubusercontent.com/Volodichev/proxy-list/main/http.txt",
    "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/http.txt",
    "https://raw.githubusercontent.com/roma8ok/proxy-list/main/proxy-list-http.txt",
];

const SOURCE_TIMEOUT: Duration = Duration::from_secs(30);

/// Download every source (concurrently) and merge them, without duplicates.
/// Sources that fail are skipped; the result may be empty.
pub async fn fetch_pool(http: &Http) -> Vec<String> {
    let fetches = SOURCES.iter().map(|source| async move {
        let response = http.get(*source).timeout(SOURCE_TIMEOUT).send().await;
        match response {
            Ok(response) if response.status().is_success() => response.text().await.ok(),
            Ok(response) => {
                tracing::warn!(source, status = %response.status(), "proxy source refused");
                None
            }
            Err(error) => {
                tracing::warn!(source, %error, "proxy source unreachable");
                None
            }
        }
    });

    let mut pool: Vec<String> = Vec::new();
    for text in join_all(fetches).await.into_iter().flatten() {
        for proxy in parse_list(&text) {
            if !pool.contains(&proxy) {
                pool.push(proxy);
            }
        }
    }
    pool
}

/// Turn "ip:port" lines into `http://ip:port` URLs, skipping anything else.
pub fn parse_list(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|entry| is_host_port(entry))
        .map(|entry| format!("http://{entry}"))
        .collect()
}

fn is_host_port(entry: &str) -> bool {
    let Some((host, port)) = entry.rsplit_once(':') else {
        return false;
    };
    !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && port.parse::<u16>().is_ok_and(|p| p > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ip_port_lines_and_skips_junk() {
        let text = "1.2.3.4:8080\n\n  5.6.7.8:3128 \nnot a proxy\nhttp://x:1\n9.9.9.9:0\n10.0.0.1:99999\nproxy.example.com:80\n";
        assert_eq!(
            parse_list(text),
            [
                "http://1.2.3.4:8080",
                "http://5.6.7.8:3128",
                "http://proxy.example.com:80"
            ]
        );
    }

    #[test]
    fn crlf_line_endings_are_fine() {
        assert_eq!(parse_list("1.1.1.1:80\r\n2.2.2.2:81\r\n").len(), 2);
    }
}

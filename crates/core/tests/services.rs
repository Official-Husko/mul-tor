//! User-agent cache, proxy pool, availability check and update check, against a mock server.

mod common;

use std::sync::Arc;

use common::*;
use mul_tor_core::availability::{self, Health};
use mul_tor_core::update;
use mul_tor_core::{Http, HttpFactory, ProxyMode, Registry, proxies, useragents};
use serde_json::json;

fn http_for(mock: &Mock) -> Http {
    HttpFactory::new()
        .with_proxy(ProxyMode::Direct)
        .with_url_rewrite(mock.rewrite())
        .client()
        .unwrap()
}

// ---------------------------------------------------------------- user agents

fn ua_reply() -> Reply {
    Reply::json(json!({"data": [{"ua": "UA-one"}, {"ua": "UA-two"}, {"nope": 1}]}))
}

#[tokio::test]
async fn user_agents_are_downloaded_once_then_served_from_the_cache() {
    let mock = Mock::start(|_| ua_reply()).await;
    let http = http_for(&mock);
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache/user_agents.json");

    assert_eq!(useragents::load(&cache, &http).await, ["UA-one", "UA-two"]);
    assert_eq!(useragents::load(&cache, &http).await, ["UA-one", "UA-two"]);
    assert_eq!(mock.requests().len(), 1, "second call must hit the cache");

    // The Python version's cache layout.
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&cache).unwrap()).unwrap();
    assert_eq!(stored["user_agents"], json!(["UA-one", "UA-two"]));
    assert_eq!(stored["creation_date"].as_str().unwrap().len(), 10);
}

#[tokio::test]
async fn an_outdated_cache_is_refreshed_and_the_new_list_returned() {
    // The Python version returned nothing at all in this situation.
    let mock = Mock::start(|_| ua_reply()).await;
    let http = http_for(&mock);
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("user_agents.json");
    std::fs::write(
        &cache,
        r#"{"creation_date": "2020-01-01", "user_agents": ["old"]}"#,
    )
    .unwrap();

    assert_eq!(useragents::load(&cache, &http).await, ["UA-one", "UA-two"]);
    assert!(std::fs::read_to_string(&cache).unwrap().contains("UA-one"));
}

#[tokio::test]
async fn a_failed_download_falls_back_to_the_stale_cache_then_the_builtin_list() {
    let mock = Mock::start(|_| Reply::text("down").status(503)).await;
    let http = http_for(&mock);
    let dir = tempfile::tempdir().unwrap();

    let stale = dir.path().join("stale.json");
    std::fs::write(
        &stale,
        r#"{"creation_date": "2020-01-01", "user_agents": ["old"]}"#,
    )
    .unwrap();
    assert_eq!(useragents::load(&stale, &http).await, ["old"]);

    let nothing = dir.path().join("none.json");
    assert_eq!(
        useragents::load(&nothing, &http).await,
        useragents::backup_list()
    );
    assert!(!nothing.exists(), "a failure must not poison the cache");
}

#[tokio::test]
async fn a_corrupt_cache_is_ignored() {
    let mock = Mock::start(|_| ua_reply()).await;
    let http = http_for(&mock);
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("user_agents.json");
    std::fs::write(&cache, "{ not json").unwrap();
    assert_eq!(useragents::load(&cache, &http).await.len(), 2);
}

// -------------------------------------------------------------------- proxies

#[tokio::test]
async fn the_proxy_pool_merges_sources_and_skips_failures() {
    let mock = Mock::start(|r| match r.path.as_str() {
        p if p.contains("TheSpeedX") => Reply::text("1.1.1.1:80\n2.2.2.2:81\n"),
        p if p.contains("ShiftyTR") => Reply::text("2.2.2.2:81\r\n3.3.3.3:82\r\njunk\r\n"),
        p if p.contains("mmpx12") => Reply::text("down").status(500),
        _ => Reply::text(""),
    })
    .await;
    let pool = proxies::fetch_pool(&http_for(&mock)).await;
    assert_eq!(
        pool,
        [
            "http://1.1.1.1:80",
            "http://2.2.2.2:81",
            "http://3.3.3.3:82"
        ]
    );
    assert_eq!(mock.requests().len(), proxies::SOURCES.len());
}

// --------------------------------------------------------------- availability

#[tokio::test]
async fn availability_reports_each_provider_independently_and_in_order() {
    let mock = Mock::start(|r| match r.host.as_str() {
        "catbox.moe" => Reply::html("ok"),
        "litterbox.catbox.moe" => Reply::text("busy").status(503),
        "oshi.at" => Reply::text("forbidden").status(403),
        _ => Reply::not_found(),
    })
    .await;
    let registry = Registry::builtin();
    let providers = vec![
        registry.find("catbox").unwrap().clone(),
        registry.find("litterbox").unwrap().clone(),
        registry.find("oshi").unwrap().clone(),
        registry.find("keep").unwrap().clone(), // mock answers 404
    ];
    let factory = HttpFactory::new()
        .with_proxy(ProxyMode::Direct)
        .with_url_rewrite(mock.rewrite());

    let statuses = availability::check(&factory, &providers).await;

    let summary: Vec<_> = statuses
        .iter()
        .map(|s| (s.provider.info().name, s.health.clone()))
        .collect();
    assert_eq!(
        summary,
        [
            ("Catbox", Health::Up),
            (
                "LitterBox",
                Health::Down("HTTP 503 Service Unavailable".into())
            ),
            ("Oshi", Health::Up),
            ("Keep", Health::Down("HTTP 404 Not Found".into())),
        ]
    );
}

#[tokio::test]
async fn availability_reports_unreachable_hosts_as_down() {
    // Nothing listens on this port.
    let factory = HttpFactory::new()
        .with_proxy(ProxyMode::Direct)
        .with_url_rewrite(Arc::new(|_: &str| "http://127.0.0.1:1/".to_string()));
    let provider = Registry::builtin().find("catbox").unwrap().clone();
    let statuses = availability::check(&factory, &[provider]).await;
    assert_eq!(statuses[0].health, Health::Down("connection failed".into()));
}

// --------------------------------------------------------------------- update

#[tokio::test]
async fn the_latest_release_is_parsed() {
    let mock = Mock::start(|r| {
        if r.is(
            "GET",
            "api.github.com",
            "/repos/Official-Husko/mul-tor/releases/latest",
        ) {
            Reply::json(json!({
                "tag_name": "v2.1.0",
                "name": "Mul-Tor 2.1",
                "body": "- faster\n- more providers",
                "published_at": "2026-03-04T05:06:07Z",
                "html_url": "https://github.com/Official-Husko/mul-tor/releases/tag/v2.1.0",
                "assets": []
            }))
        } else {
            Reply::not_found()
        }
    })
    .await;
    let release = update::latest_release(&http_for(&mock)).await.unwrap();
    assert_eq!(release.version.to_string(), "2.1.0");
    assert_eq!(release.name, "Mul-Tor 2.1");
    assert_eq!(release.published, "2026-03-04 05:06:07");
    assert!(update::is_newer("2.0.0", &release));
    assert!(!update::is_newer("2.1.0", &release));

    let request = &mock.requests()[0];
    assert_eq!(
        request.header("accept"),
        Some("application/vnd.github+json")
    );
    assert!(
        request
            .header("user-agent")
            .unwrap()
            .starts_with("mul-tor/")
    );
}

#[tokio::test]
async fn update_check_errors_are_reported_not_panicked() {
    for reply in [
        Reply::text("rate limited").status(403),
        Reply::text("not json"),
        Reply::json(json!({"tag_name": "nightly", "html_url": "x"})),
    ] {
        let mock = Mock::start(move |_| reply.clone()).await;
        assert!(update::latest_release(&http_for(&mock)).await.is_err());
    }
}

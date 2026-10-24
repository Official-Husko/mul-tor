//! Building clients.

use super::*;

#[test]
fn client_builds_for_every_proxy_mode() {
    for mode in [
        ProxyMode::System,
        ProxyMode::Direct,
        ProxyMode::Fixed("socks5h://127.0.0.1:9050".into()),
        ProxyMode::Pool(vec!["http://127.0.0.1:8080".into()]),
        ProxyMode::Pool(vec![]),
    ] {
        HttpFactory::new().with_proxy(mode).client().unwrap();
    }
}

#[test]
fn invalid_proxy_url_is_an_error_not_a_panic() {
    let result = HttpFactory::new()
        .with_proxy(ProxyMode::Fixed("not a url".into()))
        .client();
    assert!(result.is_err());
}

#[test]
fn user_agent_comes_from_the_configured_list() {
    let http = HttpFactory::new()
        .with_user_agents(vec!["UA/1".into()])
        .client()
        .unwrap();
    assert_eq!(http.user_agent(), "UA/1");
}

#[test]
fn empty_user_agent_list_keeps_the_default() {
    let http = HttpFactory::new()
        .with_user_agents(vec![])
        .client()
        .unwrap();
    assert!(http.user_agent().starts_with("mul-tor/"));
}

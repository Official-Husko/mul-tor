//! Choosing providers: unknown names and presets.

use std::sync::Arc;

use crate::common::*;

#[tokio::test(flavor = "multi_thread")]
async fn unknown_providers_are_a_usage_error() {
    let env = Arc::new(Env::new());
    let mock = Arc::new(Mock::start().await);
    let file = env.file("a.txt", "hello");

    let output = run(
        &env,
        &mock,
        args(&["upload", &path_arg(&file), "-p", "catbox,nonesuch"]),
    )
    .await;

    assert_eq!(code(&output), 2);
    assert!(
        stderr(&output).contains("unknown provider: nonesuch"),
        "{}",
        stderr(&output)
    );
    assert!(
        mock.hosts().is_empty(),
        "nothing may be uploaded when the request is invalid"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_preset_selects_its_providers_and_writes_formatted_links() {
    let env = Arc::new(Env::new());
    let mock = Arc::new(Mock::start().await);
    std::fs::create_dir_all(env.home().join("presets")).unwrap();
    std::fs::write(
        env.home().join("presets/mini.json"),
        r#"{"identifier": "me.mini", "name": "Mini", "sites": ["Catbox", "SomethingElse"],
            "link-format": "[URL='{file_url}']{provider_name}[/URL]"}"#,
    )
    .unwrap();
    let file = env.file("a.txt", "hello");

    let output = run(
        &env,
        &mock,
        args(&["upload", &path_arg(&file), "--preset", "mini", "--json"]),
    )
    .await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stderr(&output).contains("SomethingElse"),
        "unsupported names are reported"
    );
    let formatted = std::fs::read_to_string(env.cwd().join("file_links_formatted.txt")).unwrap();
    assert_eq!(
        formatted.trim(),
        "[URL='https://files.catbox.moe/file1.bin']Catbox[/URL]"
    );
}

//! Uploading from the command line: output, exit codes, files and folders.

use std::sync::Arc;

use crate::common::*;

#[tokio::test(flavor = "multi_thread")]
async fn json_mode_prints_one_object_per_upload_and_sets_the_exit_code() {
    let env = Arc::new(Env::new());
    let mock = Arc::new(Mock::start().await);
    let file = env.file("a.txt", "hello");

    let output = run(
        &env,
        &mock,
        vec![
            "upload".into(),
            path_arg(&file),
            "--provider".into(),
            "catbox,litterbox".into(),
            "--json".into(),
        ],
    )
    .await;

    assert_eq!(code(&output), 1, "one provider failed: {}", stderr(&output));
    let lines: Vec<serde_json::Value> = stdout(&output)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON: {l:?} ({e})")))
        .collect();
    assert_eq!(lines.len(), 2, "{}", stdout(&output));
    assert_eq!(lines[0]["provider"], "Catbox");
    assert_eq!(lines[0]["status"], "ok");
    assert_eq!(lines[0]["url"], "https://files.catbox.moe/file1.bin");
    assert_eq!(lines[1]["provider"], "LitterBox");
    assert_eq!(lines[1]["status"], "error");
    assert_eq!(lines[1]["kind"], "protocol");

    // Only the successful upload is recorded.
    let links = std::fs::read_to_string(env.cwd().join("file_links.txt")).unwrap();
    assert_eq!(links.lines().count(), 1);
    assert!(links.contains("| Catbox | a.txt - https://files.catbox.moe/file1.bin"));
}

#[tokio::test(flavor = "multi_thread")]
async fn human_output_reports_links_and_a_summary() {
    let env = Arc::new(Env::new());
    let mock = Arc::new(Mock::start().await);
    let file = env.file("a.txt", "hello");

    let output = run(
        &env,
        &mock,
        vec![
            "upload".into(),
            path_arg(&file),
            "-p".into(),
            "catbox".into(),
        ],
    )
    .await;

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("a.txt uploaded to Catbox! URL: https://files.catbox.moe/file1.bin"),
        "{out}"
    );
    assert!(out.contains("1 uploaded"), "{out}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failing_upload_is_explained_and_logged() {
    let env = Arc::new(Env::new());
    let mock = Arc::new(Mock::start().await);
    let file = env.file("a.txt", "hello");

    let output = run(
        &env,
        &mock,
        vec![
            "upload".into(),
            path_arg(&file),
            "-p".into(),
            "litterbox".into(),
        ],
    )
    .await;

    assert_eq!(code(&output), 1);
    let out = stdout(&output);
    assert!(out.contains("a.txt to LitterBox failed:"), "{out}");
    assert!(out.contains("1 failed"), "{out}");
    let log_file = mul_tor_core::logging::latest_log(&env.home().join("logs"))
        .expect("the run wrote a log file");
    let log = std::fs::read_to_string(log_file).unwrap();
    assert!(
        log.contains("upload failed") && log.contains("LitterBox"),
        "{log}"
    );
    assert!(
        log.contains("litterbox is down"),
        "the raw reply belongs in the log: {log}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_credentials_fail_the_job_before_any_request() {
    let env = Arc::new(Env::new());
    let mock = Arc::new(Mock::start().await);
    let file = env.file("a.txt", "hello");

    let output = run(
        &env,
        &mock,
        args(&["upload", &path_arg(&file), "-p", "pixeldrain", "--json"]),
    )
    .await;

    assert_eq!(code(&output), 1);
    let line: serde_json::Value =
        serde_json::from_str(stdout(&output).lines().next().unwrap()).unwrap();
    assert_eq!(line["kind"], "missing_credential");
    assert!(mock.hosts().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn folders_expand_and_a_missing_path_fails_the_run_but_not_the_rest() {
    let env = Arc::new(Env::new());
    let mock = Arc::new(Mock::start().await);
    env.file("pics/one.txt", "1");
    env.file("pics/two.txt", "2");
    env.file("pics/deep/three.txt", "3");
    let pics = env.cwd().join("pics");
    let missing = env.cwd().join("nope.txt");

    let output = run(
        &env,
        &mock,
        args(&[
            "upload",
            &path_arg(&pics),
            &path_arg(&missing),
            "-p",
            "catbox",
            "--json",
            "-j",
            "2",
        ]),
    )
    .await;

    assert_eq!(code(&output), 1, "a missing input must fail the run");
    assert!(stderr(&output).contains("nope.txt"), "{}", stderr(&output));
    let mut uploaded: Vec<String> = stdout(&output)
        .lines()
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).unwrap()["file"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    uploaded.sort();
    assert_eq!(
        uploaded,
        ["one.txt", "two.txt"],
        "sub-folders need --recursive"
    );

    let recursive = run(
        &env,
        &mock,
        args(&["upload", &path_arg(&pics), "-p", "catbox", "--json", "-r"]),
    )
    .await;
    assert_eq!(code(&recursive), 0);
    assert_eq!(stdout(&recursive).lines().count(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn links_can_go_elsewhere_or_nowhere() {
    let env = Arc::new(Env::new());
    let mock = Arc::new(Mock::start().await);
    let file = env.file("a.txt", "hello");
    let elsewhere = env.root.path().join("elsewhere");

    run(
        &env,
        &mock,
        args(&[
            "upload",
            &path_arg(&file),
            "-p",
            "catbox",
            "--links-dir",
            &path_arg(&elsewhere),
        ]),
    )
    .await;
    assert!(elsewhere.join("file_links.txt").exists());
    assert!(!env.cwd().join("file_links.txt").exists());

    run(
        &env,
        &mock,
        args(&[
            "upload",
            &path_arg(&file),
            "-p",
            "catbox",
            "--no-save-links",
        ]),
    )
    .await;
    assert!(!env.cwd().join("file_links.txt").exists());
}

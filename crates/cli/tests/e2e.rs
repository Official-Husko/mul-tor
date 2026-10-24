//! End-to-end tests: run the real `mul-tor` binary against a local mock server.
//!
//! Needs the `test-hooks` feature, which lets `MUL_TOR_TEST_BASE_URL` redirect
//! every site to localhost. Without the hook these tests would talk to the real
//! sites, so the whole file is compiled out unless it is enabled:
//!
//!     cargo test -p mul-tor --features test-hooks
#![cfg(feature = "test-hooks")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, Response, StatusCode};

/// A tiny site stand-in: Catbox works, LitterBox fails, everything else is 404.
struct Mock {
    base: String,
    requests: Arc<Mutex<Vec<(String, String)>>>, // (host, path)
    _task: tokio::task::JoinHandle<()>,
}

impl Mock {
    async fn start() -> Mock {
        let requests: Arc<Mutex<Vec<(String, String)>>> = Arc::default();
        let log = requests.clone();
        let counter = Arc::new(Mutex::new(0u32));
        let app = Router::new().fallback(move |request: Request<Body>| {
            let log = log.clone();
            let counter = counter.clone();
            async move {
                let (parts, body) = request.into_parts();
                let _ = to_bytes(body, usize::MAX).await;
                let full = parts.uri.path().trim_start_matches('/').to_string();
                let (host, path) = full.split_once('/').unwrap_or((&full, ""));
                log.lock()
                    .unwrap()
                    .push((host.to_string(), format!("/{path}")));
                let (status, body) = match host {
                    "catbox.moe" => {
                        let mut n = counter.lock().unwrap();
                        *n += 1;
                        (200, format!("https://files.catbox.moe/file{n}.bin"))
                    }
                    "litterbox.catbox.moe" => (500, "litterbox is down".to_string()),
                    _ => (404, "no such site".to_string()),
                };
                Response::builder()
                    .status(StatusCode::from_u16(status).unwrap())
                    .body(Body::from(body))
                    .unwrap()
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Mock {
            base,
            requests,
            _task: task,
        }
    }

    fn hosts(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|(h, _)| h.clone())
            .collect()
    }
}

/// A scratch home folder with a quiet config, plus a working directory.
struct Env {
    root: tempfile::TempDir,
}

impl Env {
    fn new() -> Self {
        let env = Env {
            root: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(env.home()).unwrap();
        std::fs::create_dir_all(env.cwd()).unwrap();
        std::fs::write(
            env.home().join("config.toml"),
            "check_for_updates = false\ncheck_availability = false\n",
        )
        .unwrap();
        env
    }

    fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    fn cwd(&self) -> PathBuf {
        self.root.path().join("cwd")
    }

    fn file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.cwd().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        path
    }

    fn run(&self, mock: Option<&Mock>, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mul-tor"));
        command
            .args(args)
            .current_dir(self.cwd())
            .env("MUL_TOR_HOME", self.home())
            .env("NO_COLOR", "1")
            .env_remove("HTTPS_PROXY")
            .env_remove("https_proxy");
        if let Some(mock) = mock {
            command.env("MUL_TOR_TEST_BASE_URL", &mock.base);
        }
        command.output().unwrap()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

/// Run the CLI off the async runtime so the mock server keeps serving.
async fn run(env: &Arc<Env>, mock: &Arc<Mock>, args: Vec<String>) -> Output {
    let (env, mock) = (env.clone(), mock.clone());
    tokio::task::spawn_blocking(move || {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        env.run(Some(&mock), &args)
    })
    .await
    .unwrap()
}

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

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
            "--site".into(),
            "catbox,litterbox".into(),
            "--json".into(),
        ],
    )
    .await;

    assert_eq!(code(&output), 1, "one site failed: {}", stderr(&output));
    let lines: Vec<serde_json::Value> = stdout(&output)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON: {l:?} ({e})")))
        .collect();
    assert_eq!(lines.len(), 2, "{}", stdout(&output));
    assert_eq!(lines[0]["site"], "Catbox");
    assert_eq!(lines[0]["status"], "ok");
    assert_eq!(lines[0]["url"], "https://files.catbox.moe/file1.bin");
    assert_eq!(lines[1]["site"], "LitterBox");
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
            "-s".into(),
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
            "-s".into(),
            "litterbox".into(),
        ],
    )
    .await;

    assert_eq!(code(&output), 1);
    let out = stdout(&output);
    assert!(out.contains("a.txt to LitterBox failed:"), "{out}");
    assert!(out.contains("1 failed"), "{out}");
    let log = std::fs::read_to_string(env.home().join("runtime.log")).unwrap();
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
        args(&["upload", &path_arg(&file), "-s", "pixeldrain", "--json"]),
    )
    .await;

    assert_eq!(code(&output), 1);
    let line: serde_json::Value =
        serde_json::from_str(stdout(&output).lines().next().unwrap()).unwrap();
    assert_eq!(line["kind"], "missing_credential");
    assert!(mock.hosts().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_sites_are_a_usage_error() {
    let env = Arc::new(Env::new());
    let mock = Arc::new(Mock::start().await);
    let file = env.file("a.txt", "hello");

    let output = run(
        &env,
        &mock,
        args(&["upload", &path_arg(&file), "-s", "catbox,nonesuch"]),
    )
    .await;

    assert_eq!(code(&output), 2);
    assert!(
        stderr(&output).contains("unknown site: nonesuch"),
        "{}",
        stderr(&output)
    );
    assert!(
        mock.hosts().is_empty(),
        "nothing may be uploaded when the request is invalid"
    );
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
            "-s",
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
        args(&["upload", &path_arg(&pics), "-s", "catbox", "--json", "-r"]),
    )
    .await;
    assert_eq!(code(&recursive), 0);
    assert_eq!(stdout(&recursive).lines().count(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_preset_selects_its_sites_and_writes_formatted_links() {
    let env = Arc::new(Env::new());
    let mock = Arc::new(Mock::start().await);
    std::fs::create_dir_all(env.home().join("presets")).unwrap();
    std::fs::write(
        env.home().join("presets/mini.json"),
        r#"{"identifier": "me.mini", "name": "Mini", "sites": ["Catbox", "SomethingElse"],
            "link-format": "[URL='{file_url}']{site_name}[/URL]"}"#,
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
            "-s",
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
            "-s",
            "catbox",
            "--no-save-links",
        ]),
    )
    .await;
    assert!(!env.cwd().join("file_links.txt").exists());
}

#[test]
fn the_python_config_in_the_working_directory_is_imported_with_secrets_masked() {
    let env = Env::new();
    std::fs::remove_file(env.home().join("config.toml")).unwrap();
    std::fs::write(
        env.cwd().join("config.json"),
        r#"{"checkForUpdates": false, "api_keys": {"Pixeldrain": {"apiKey": "super-secret-key"}}}"#,
    )
    .unwrap();

    let shown = env.run(None, &["config", "show"]);

    assert_eq!(code(&shown), 0, "{}", stderr(&shown));
    assert!(
        stderr(&shown).contains("Imported your old config.json"),
        "{}",
        stderr(&shown)
    );
    let out = stdout(&shown);
    assert!(
        out.contains("[credentials.Pixeldrain]") && out.contains("********"),
        "{out}"
    );
    assert!(!out.contains("super-secret-key"));
    assert!(
        std::fs::read_to_string(env.home().join("config.toml"))
            .unwrap()
            .contains("super-secret-key")
    );
    assert!(
        env.cwd().join("config.json").exists(),
        "the old file is left alone"
    );
}

#[test]
fn interactive_mode_refuses_to_run_without_a_terminal() {
    let env = Env::new();
    let output = env.run(None, &[]);
    assert_eq!(code(&output), 2);
    assert!(
        stderr(&output).contains("needs a terminal"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn informational_commands_work_offline() {
    let env = Env::new();
    for command in [
        &["sites"][..],
        &["sites", "--format", "markdown"],
        &["sites", "--format", "json"],
        &["presets"],
        &["config", "path"],
    ] {
        let output = env.run(None, command);
        assert_eq!(code(&output), 0, "{command:?}: {}", stderr(&output));
        assert!(!stdout(&output).is_empty(), "{command:?}");
    }
    let sites = env.run(None, &["sites", "--format", "json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&sites)).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 37);
}

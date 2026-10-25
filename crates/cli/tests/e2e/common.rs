//! The harness: a mock provider server, a scratch home folder, and helpers to
//! run the real binary against them.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, Response, StatusCode};

/// A tiny provider stand-in: Catbox works, LitterBox fails, everything else is 404.
pub struct Mock {
    pub base: String,
    pub requests: Arc<Mutex<Vec<(String, String)>>>, // (host, path)
    pub _task: tokio::task::JoinHandle<()>,
}

impl Mock {
    pub async fn start() -> Mock {
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
                    _ => (404, "no such provider".to_string()),
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

    pub fn hosts(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|(h, _)| h.clone())
            .collect()
    }
}

/// A scratch home folder with a quiet config, plus a working directory.
pub struct Env {
    pub root: tempfile::TempDir,
}

impl Env {
    pub fn new() -> Self {
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

    pub fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    pub fn cwd(&self) -> PathBuf {
        self.root.path().join("cwd")
    }

    pub fn file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.cwd().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        path
    }

    pub fn run(&self, mock: Option<&Mock>, args: &[&str]) -> Output {
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

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

/// Run the CLI off the async runtime so the mock server keeps serving.
pub async fn run(env: &Arc<Env>, mock: &Arc<Mock>, args: Vec<String>) -> Output {
    let (env, mock) = (env.clone(), mock.clone());
    tokio::task::spawn_blocking(move || {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        env.run(Some(&mock), &args)
    })
    .await
    .unwrap()
}

pub fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

pub fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

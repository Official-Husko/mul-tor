//! Fake providers and a harness, so scheduling can be controlled.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use mul_tor_core::{
    CredentialField, Engine, Event, EventSink, HttpFactory, JobOutcome, Provider, ProviderInfo,
    ProxyMode, UploadContext, UploadError, UploadedFile,
};

#[derive(Clone, Copy)]
pub enum Behaviour {
    Succeed,
    Fail,
    Hang,
    Panic,
    /// Takes about `ms` milliseconds, reporting progress along the way.
    Work(u64),
    /// Succeeds with the credential `apiKey` as the "link".
    EchoKey,
}

#[derive(Default)]
pub struct Probe {
    pub calls: AtomicUsize,
    pub active: AtomicUsize,
    pub max_active: AtomicUsize,
    pub started: Mutex<Vec<String>>,
}

pub struct ActiveGuard<'a>(&'a Probe);

impl Drop for ActiveGuard<'_> {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct Fake {
    pub info: ProviderInfo,
    pub behaviour: Behaviour,
    pub probe: Arc<Probe>,
}

pub fn fake(info: ProviderInfo, behaviour: Behaviour, probe: &Arc<Probe>) -> Arc<dyn Provider> {
    Arc::new(Fake {
        info,
        behaviour,
        probe: probe.clone(),
    })
}

/// Must be a `const` so the slice is `'static`.
pub const BLOCKED: &[&str] = &[".exe"];
pub const API_KEY: &[CredentialField] = &[CredentialField::new("apiKey", "key", true)];

pub fn plain(name: &'static str) -> ProviderInfo {
    ProviderInfo::new(name, "https://fake.example/", "https://fake.example/")
}

#[async_trait]
impl Provider for Fake {
    fn info(&self) -> &ProviderInfo {
        &self.info
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let probe = &*self.probe;
        probe.calls.fetch_add(1, Ordering::SeqCst);
        let now = probe.active.fetch_add(1, Ordering::SeqCst) + 1;
        probe.max_active.fetch_max(now, Ordering::SeqCst);
        probe
            .started
            .lock()
            .unwrap()
            .push(format!("{}:{}", self.info.name, cx.file.name));
        let _guard = ActiveGuard(probe);

        match self.behaviour {
            Behaviour::Succeed => Ok(UploadedFile::new(format!(
                "https://fake.example/{}",
                cx.file.name
            ))),
            Behaviour::Fail => Err(UploadError::protocol("nope", "raw reply")),
            Behaviour::Hang => {
                std::future::pending::<()>().await;
                unreachable!()
            }
            Behaviour::Panic => panic!("uploader bug"),
            Behaviour::Work(ms) => {
                for _ in 0..10 {
                    tokio::time::sleep(Duration::from_millis(ms / 10)).await;
                    cx.progress.add(cx.file.size / 10);
                }
                Ok(UploadedFile::new("https://fake.example/done"))
            }
            Behaviour::EchoKey => Ok(UploadedFile::new(cx.credential("apiKey")?.to_string())),
        }
    }
}

pub struct Harness {
    pub dir: tempfile::TempDir,
    pub events: Arc<Mutex<Vec<Event>>>,
    pub sink: EventSink,
}

impl Harness {
    pub fn new() -> Self {
        let events: Arc<Mutex<Vec<Event>>> = Arc::default();
        let log = events.clone();
        Self {
            dir: tempfile::tempdir().unwrap(),
            events,
            sink: Arc::new(move |event| log.lock().unwrap().push(event)),
        }
    }

    pub fn file(&self, name: &str, content: &[u8]) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    pub fn engine(&self) -> Engine {
        Engine::new(HttpFactory::new().with_proxy(ProxyMode::Direct))
    }

    pub fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }
}

pub fn finished(outcomes: &[Arc<JobOutcome>]) -> Vec<(String, &'static str, bool)> {
    outcomes
        .iter()
        .map(|o| (o.file_name.clone(), o.provider, o.is_ok()))
        .collect()
}

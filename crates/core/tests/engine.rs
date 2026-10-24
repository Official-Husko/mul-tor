//! Engine behaviour, using fake sites so scheduling can be controlled.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use mul_tor_core::size::SizeLimit;
use mul_tor_core::{
    CredentialField, Credentials, Engine, Event, EventSink, HttpFactory, JobOutcome, LinkLog,
    ProxyMode, Site, SiteInfo, UploadContext, UploadError, UploadedFile,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy)]
enum Behaviour {
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
struct Probe {
    calls: AtomicUsize,
    active: AtomicUsize,
    max_active: AtomicUsize,
    started: Mutex<Vec<String>>,
}

struct ActiveGuard<'a>(&'a Probe);

impl Drop for ActiveGuard<'_> {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}

struct Fake {
    info: SiteInfo,
    behaviour: Behaviour,
    probe: Arc<Probe>,
}

fn fake(info: SiteInfo, behaviour: Behaviour, probe: &Arc<Probe>) -> Arc<dyn Site> {
    Arc::new(Fake {
        info,
        behaviour,
        probe: probe.clone(),
    })
}

/// Must be a `const` so the slice is `'static`.
const BLOCKED: &[&str] = &[".exe"];
const API_KEY: &[CredentialField] = &[CredentialField::new("apiKey", "key", true)];

fn plain(name: &'static str) -> SiteInfo {
    SiteInfo::new(name, "https://fake.example/", "https://fake.example/")
}

#[async_trait]
impl Site for Fake {
    fn info(&self) -> &SiteInfo {
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

struct Harness {
    dir: tempfile::TempDir,
    events: Arc<Mutex<Vec<Event>>>,
    sink: EventSink,
}

impl Harness {
    fn new() -> Self {
        let events: Arc<Mutex<Vec<Event>>> = Arc::default();
        let log = events.clone();
        Self {
            dir: tempfile::tempdir().unwrap(),
            events,
            sink: Arc::new(move |event| log.lock().unwrap().push(event)),
        }
    }

    fn file(&self, name: &str, content: &[u8]) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn engine(&self) -> Engine {
        Engine::new(HttpFactory::new().with_proxy(ProxyMode::Direct))
    }

    fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }
}

fn finished(outcomes: &[Arc<JobOutcome>]) -> Vec<(String, &'static str, bool)> {
    outcomes
        .iter()
        .map(|o| (o.file_name.clone(), o.site, o.is_ok()))
        .collect()
}

#[tokio::test]
async fn jobs_run_file_major_in_the_order_given() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let sites = vec![
        fake(plain("A"), Behaviour::Succeed, &probe),
        fake(plain("B"), Behaviour::Succeed, &probe),
    ];
    let files = vec![h.file("one", b"1"), h.file("two", b"2")];

    let outcomes = h
        .engine()
        .run(files, sites, h.sink.clone(), CancellationToken::new())
        .await;

    assert_eq!(
        finished(&outcomes),
        [
            ("one".into(), "A", true),
            ("one".into(), "B", true),
            ("two".into(), "A", true),
            ("two".into(), "B", true),
        ]
    );
    assert_eq!(
        *probe.started.lock().unwrap(),
        ["A:one", "B:one", "A:two", "B:two"]
    );
    assert_eq!(
        probe.max_active.load(Ordering::SeqCst),
        1,
        "default is sequential"
    );
}

#[tokio::test]
async fn every_job_is_queued_before_any_starts_and_finishes_last() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let sites = vec![fake(plain("A"), Behaviour::Succeed, &probe)];
    h.engine()
        .run(
            vec![h.file("f1", b"x"), h.file("f2", b"y")],
            sites,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    let events = h.events();
    let first_started = events
        .iter()
        .position(|e| matches!(e, Event::Started { .. }))
        .unwrap();
    let queued = events
        .iter()
        .filter(|e| matches!(e, Event::Queued { .. }))
        .count();
    assert_eq!(queued, 2);
    assert!(
        events[..first_started]
            .iter()
            .all(|e| matches!(e, Event::Queued { .. }))
    );

    for job in 0..2 {
        let kinds: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                Event::Started { job: j } if *j == job => Some("started"),
                Event::Finished(o) if o.job == job => Some("finished"),
                _ => None,
            })
            .collect();
        assert_eq!(kinds, ["started", "finished"]);
    }
}

#[tokio::test]
async fn parallelism_is_bounded_by_the_setting() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let sites: Vec<_> = ["A", "B", "C", "D"]
        .into_iter()
        .map(|name| fake(plain(name), Behaviour::Work(200), &probe))
        .collect();

    let outcomes = h
        .engine()
        .with_parallelism(2)
        .run(
            vec![h.file("f", b"0123456789")],
            sites,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    assert!(outcomes.iter().all(|o| o.is_ok()));
    assert_eq!(probe.max_active.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn offline_checks_stop_a_job_before_the_site_is_called() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let small = plain("Small").max_size(SizeLimit::mb(1));
    let keyed = plain("Keyed").requires(API_KEY);
    let picky = plain("Picky").blocked_extensions(BLOCKED);
    let sites = vec![
        fake(small, Behaviour::Succeed, &probe),
        fake(keyed, Behaviour::Succeed, &probe),
        fake(picky, Behaviour::Succeed, &probe),
    ];
    let big = h.dir.path().join("big.exe");
    std::fs::File::create(&big)
        .unwrap()
        .set_len(2 * 1024 * 1024)
        .unwrap();
    let empty = h.file("empty.bin", b"");

    let outcomes = h
        .engine()
        .run(
            vec![big, empty],
            sites,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    let errors: Vec<String> = outcomes
        .iter()
        .map(|o| match &o.result {
            Err(UploadError::TooLarge { .. }) => "too large".into(),
            Err(UploadError::MissingCredential(k)) => format!("missing {k}"),
            Err(UploadError::Rejected(_)) => "rejected".into(),
            Err(UploadError::EmptyFile) => "empty".into(),
            other => format!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        errors,
        [
            "too large",
            "missing apiKey",
            "rejected",
            "empty",
            "empty",
            "empty"
        ]
    );
    assert_eq!(probe.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn credentials_reach_the_site_by_site_id() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let keyed = plain("Key.d").requires(API_KEY);
    let sites = vec![fake(keyed, Behaviour::EchoKey, &probe)];

    let mut creds = Credentials::default();
    creds.set("apiKey", "SECRET-1");
    let engine = h
        .engine()
        .with_credentials([("keyd".to_string(), creds)].into());

    let outcomes = engine
        .run(
            vec![h.file("f", b"x")],
            sites,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(outcomes[0].result.as_ref().unwrap().url, "SECRET-1");
}

#[tokio::test]
async fn an_unreadable_file_fails_its_jobs_but_not_the_rest() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let sites = vec![
        fake(plain("A"), Behaviour::Succeed, &probe),
        fake(plain("B"), Behaviour::Succeed, &probe),
    ];
    let missing = h.dir.path().join("missing.bin");
    let good = h.file("good.bin", b"data");

    let outcomes = h
        .engine()
        .run(
            vec![missing, good],
            sites,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    assert_eq!(
        finished(&outcomes),
        [
            ("missing.bin".into(), "A", false),
            ("missing.bin".into(), "B", false),
            ("good.bin".into(), "A", true),
            ("good.bin".into(), "B", true),
        ]
    );
    assert!(matches!(outcomes[0].result, Err(UploadError::Io(_))));
}

#[tokio::test]
async fn cancelling_aborts_running_jobs_and_skips_the_rest() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let sites = vec![
        fake(plain("Hangs"), Behaviour::Hang, &probe),
        fake(plain("Later"), Behaviour::Succeed, &probe),
    ];
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    let probe_for_trigger = probe.clone();
    tokio::spawn(async move {
        while probe_for_trigger.calls.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        trigger.cancel();
    });

    let outcomes = tokio::time::timeout(
        Duration::from_secs(10),
        h.engine()
            .run(vec![h.file("f", b"x")], sites, h.sink.clone(), cancel),
    )
    .await
    .expect("run must not hang after cancellation");

    assert_eq!(outcomes.len(), 2);
    assert!(
        outcomes
            .iter()
            .all(|o| matches!(o.result, Err(UploadError::Cancelled)))
    );
    assert_eq!(
        probe.calls.load(Ordering::SeqCst),
        1,
        "the second site never started"
    );
    assert_eq!(
        probe.active.load(Ordering::SeqCst),
        0,
        "the hung upload was dropped"
    );

    // Every job still gets exactly one Finished event.
    let finishes = h
        .events()
        .iter()
        .filter(|e| matches!(e, Event::Finished(_)))
        .count();
    assert_eq!(finishes, 2);
}

#[tokio::test]
async fn cancelling_before_the_run_starts_nothing() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let cancel = CancellationToken::new();
    cancel.cancel();
    let outcomes = h
        .engine()
        .run(
            vec![h.file("f", b"x")],
            vec![fake(plain("A"), Behaviour::Succeed, &probe)],
            h.sink.clone(),
            cancel,
        )
        .await;
    assert!(matches!(outcomes[0].result, Err(UploadError::Cancelled)));
    assert_eq!(probe.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_panicking_uploader_becomes_a_failed_job() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let sites = vec![
        fake(plain("Boom"), Behaviour::Panic, &probe),
        fake(plain("Fine"), Behaviour::Succeed, &probe),
    ];
    let outcomes = h
        .engine()
        .run(
            vec![h.file("f", b"x")],
            sites,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(
        finished(&outcomes),
        [("f".into(), "Boom", false), ("f".into(), "Fine", true)]
    );
    assert!(
        outcomes[0]
            .result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("crashed")
    );
    assert_eq!(
        h.events()
            .iter()
            .filter(|e| matches!(e, Event::Finished(_)))
            .count(),
        2
    );
}

#[tokio::test]
async fn progress_is_reported_while_a_job_runs() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let sites = vec![fake(plain("Slow"), Behaviour::Work(600), &probe)];
    h.engine()
        .run(
            vec![h.file("f", &[0u8; 1000])],
            sites,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    let sent: Vec<u64> = h
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Progress { progress, .. } => Some(progress.sent),
            _ => None,
        })
        .collect();
    assert!(sent.len() >= 3, "expected several samples, got {sent:?}");
    assert!(
        sent.windows(2).all(|w| w[0] <= w[1]),
        "progress went backwards: {sent:?}"
    );
    assert_eq!(*sent.last().unwrap(), 1000);
    let totals: Vec<u64> = h
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Progress { progress, .. } => Some(progress.total),
            _ => None,
        })
        .collect();
    assert!(totals.iter().all(|t| *t == 1000));
}

#[tokio::test]
async fn only_successful_uploads_are_written_to_the_link_file() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let sites = vec![
        fake(plain("Good"), Behaviour::Succeed, &probe),
        fake(plain("Bad"), Behaviour::Fail, &probe),
    ];
    let out = h.dir.path().join("links");
    let log = LinkLog::new(&out).with_template(Some("[{site_name}]({file_url})".into()));

    h.engine()
        .with_link_log(Some(log))
        .run(
            vec![h.file("a.zip", b"x")],
            sites,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    let plain_log = std::fs::read_to_string(out.join("file_links.txt")).unwrap();
    assert_eq!(plain_log.lines().count(), 1);
    assert!(plain_log.contains("| Good | a.zip - https://fake.example/a.zip"));
    let formatted = std::fs::read_to_string(out.join("file_links_formatted.txt")).unwrap();
    assert_eq!(formatted.trim(), "[Good](https://fake.example/a.zip)");
}

#[tokio::test]
async fn no_files_or_no_sites_is_a_quiet_no_op() {
    let h = Harness::new();
    let outcomes = h
        .engine()
        .run(vec![], vec![], h.sink.clone(), CancellationToken::new())
        .await;
    assert!(outcomes.is_empty());
    assert!(h.events().is_empty());
}

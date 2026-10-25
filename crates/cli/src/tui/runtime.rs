//! Carrying out the commands the model asks for, and reporting back as messages.

use std::io::stdout;
use std::sync::Arc;
use std::time::Duration;

use crossterm::clipboard::CopyToClipboard;
use mul_tor_core::{Engine, EventSink, HttpFactory, LinkLog, availability, update};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use super::msg::{Cmd, Level, Msg, Plan};
use crate::app::App;

/// How long to wait for the "is there a newer version" answer.
const UPDATE_TIMEOUT: Duration = Duration::from_secs(8);

pub struct Runtime {
    app: Arc<App>,
    tx: UnboundedSender<Msg>,
    upload: Option<CancellationToken>,
}

impl Runtime {
    pub fn new(app: Arc<App>, tx: UnboundedSender<Msg>) -> Self {
        Self {
            app,
            tx,
            upload: None,
        }
    }

    pub fn exec(&mut self, commands: Vec<Cmd>) {
        for command in commands {
            match command {
                Cmd::Quit => self.cancel_upload(),
                Cmd::CancelUpload => self.cancel_upload(),
                Cmd::StartUpload(plan) => self.start_upload(plan),
                Cmd::CheckAvailability => self.check_availability(),
                Cmd::CheckUpdate => self.check_update(),
                Cmd::Copy(text) => self.copy(&text),
            }
        }
    }

    /// Stop whatever is still running; used when leaving.
    pub fn shutdown(&mut self) {
        self.cancel_upload();
    }

    fn cancel_upload(&mut self) {
        if let Some(token) = self.upload.take() {
            token.cancel();
        }
    }

    fn start_upload(&mut self, plan: Plan) {
        let cancel = CancellationToken::new();
        self.upload = Some(cancel.clone());
        let (app, tx) = (self.app.clone(), self.tx.clone());
        tokio::spawn(async move {
            run_upload(&app, plan, &tx, cancel).await;
            let _ = tx.send(Msg::UploadFinished);
        });
    }

    fn check_availability(&self) {
        let (app, tx) = (self.app.clone(), self.tx.clone());
        tokio::spawn(async move {
            let providers = app.eligible_providers();
            let results = match app.http_factory(None, true).await {
                Ok(factory) => availability::check(&factory, &providers)
                    .await
                    .into_iter()
                    .map(|status| (status.provider.info().id(), status.health))
                    .collect(),
                Err(error) => {
                    tracing::warn!(%error, "could not set up the connection for the availability check");
                    Vec::new()
                }
            };
            let _ = tx.send(Msg::Availability(results));
        });
    }

    fn check_update(&self) {
        let (app, tx) = (self.app.clone(), self.tx.clone());
        tokio::spawn(async move {
            let Ok(factory) = app.http_factory(None, true).await else {
                return;
            };
            let Ok(http) = factory.client() else { return };
            let lookup = tokio::time::timeout(UPDATE_TIMEOUT, update::latest_release(&http)).await;
            match lookup {
                Ok(Ok(release)) if update::is_newer(env!("CARGO_PKG_VERSION"), &release) => {
                    let _ = tx.send(Msg::UpdateFound(release));
                }
                Ok(Ok(_)) => {}
                // Being offline is normal; the log has it if anyone wonders.
                Ok(Err(error)) => tracing::info!(%error, "update check failed"),
                Err(_) => tracing::info!("update check timed out"),
            }
        });
    }

    /// OSC 52: the terminal puts the text on the clipboard, if it allows that.
    fn copy(&self, text: &str) {
        let lines = text.lines().count();
        let sent = crossterm::execute!(stdout(), CopyToClipboard::to_clipboard_from(text));
        let (level, message) = match sent {
            Ok(()) => (
                Level::Info,
                format!(
                    "Sent {lines} link{} to the clipboard (needs terminal support)",
                    if lines == 1 { "" } else { "s" }
                ),
            ),
            Err(error) => (Level::Error, format!("Could not copy: {error}")),
        };
        let _ = self.tx.send(Msg::Notice(level, message));
    }
}

async fn run_upload(app: &App, plan: Plan, tx: &UnboundedSender<Msg>, cancel: CancellationToken) {
    let factory: HttpFactory = match app.http_factory(None, true).await {
        Ok(factory) => factory,
        Err(error) => {
            let _ = tx.send(Msg::Notice(
                Level::Error,
                format!("Could not start: {error}"),
            ));
            return;
        }
    };
    let link_log = plan
        .save_links
        .then(|| LinkLog::new(plan.links_dir).with_template(plan.link_format));
    let engine = Engine::new(factory)
        .with_credentials(app.config.credentials_by_id())
        .with_parallelism(plan.parallel)
        .with_link_log(link_log);

    let events = tx.clone();
    let sink: EventSink = Arc::new(move |event| {
        let _ = events.send(Msg::Engine(event));
    });
    engine.run(plan.files, plan.providers, sink, cancel).await;
}

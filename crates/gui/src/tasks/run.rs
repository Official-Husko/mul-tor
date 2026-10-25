//! Running the engine in the background.

use std::path::PathBuf;
use std::sync::Arc;

use mul_tor_core::{AppDirs, Config, Engine, LinkLog, Provider, setup};
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

use super::{Bus, Level, Msg};

pub struct RunRequest {
    pub files: Vec<PathBuf>,
    pub providers: Vec<Arc<dyn Provider>>,
    pub config: Config,
    pub dirs: AppDirs,
    pub link_format: Option<String>,
    /// Test hook: points every provider at a local server. Not present in real builds.
    #[cfg(test)]
    pub url_rewrite: Option<mul_tor_core::http::UrlRewrite>,
}

/// Start uploading; returns the token that cancels the run.
pub fn start_run(rt: &Handle, bus: &Bus, request: RunRequest) -> CancellationToken {
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let bus = bus.clone();
    rt.spawn(async move {
        #[cfg(test)]
        let url_rewrite = request.url_rewrite.clone();
        let RunRequest {
            files,
            providers,
            config,
            dirs,
            link_format,
            ..
        } = request;

        let setup = setup::http_factory(&config, &dirs, None).await;
        for note in &setup.notes {
            bus.notice(Level::Info, note.clone());
        }
        #[allow(unused_mut)]
        let mut factory = setup.factory;
        // Test hook (compiled out of real builds): talk to a local server.
        #[cfg(test)]
        if let Some(rewrite) = url_rewrite {
            factory = factory
                .with_proxy(mul_tor_core::ProxyMode::Direct)
                .with_url_rewrite(rewrite);
        }

        let links_dir = config.links_dir.clone().unwrap_or_else(|| dirs.links_dir());
        let link_log = config
            .save_links
            .then(|| LinkLog::new(links_dir).with_template(link_format));
        let engine = Engine::new(factory)
            .with_credentials(config.credentials_by_id())
            .with_parallelism(config.parallel())
            .with_link_log(link_log);

        let sink_bus = bus.clone();
        let sink: mul_tor_core::EventSink =
            Arc::new(move |event| sink_bus.send(Msg::Engine(event)));
        engine.run(files, providers, sink, token).await;
        bus.send(Msg::RunFinished);
    });
    cancel
}

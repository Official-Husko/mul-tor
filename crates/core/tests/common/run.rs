//! Running one uploader against a [`Mock`].

use mul_tor_core::{
    Credentials, HttpFactory, LocalFile, Progress, Provider, ProxyMode, UploadContext, UploadError,
    UploadedFile,
};

use super::server::Mock;

/// Outcome of running one uploader against a [`Mock`].
pub struct Run {
    pub result: Result<UploadedFile, UploadError>,
    pub progress: Progress,
    pub file: LocalFile,
}

impl Run {
    pub fn url(&self) -> &str {
        match &self.result {
            Ok(file) => &file.url,
            Err(e) => panic!("upload failed: {e}"),
        }
    }

    pub fn error(&self) -> &UploadError {
        match &self.result {
            Ok(file) => panic!("expected an error, got {}", file.url),
            Err(e) => e,
        }
    }
}

pub struct RunConfig {
    /// The name of the file on disk.
    pub name: String,
    /// The name the uploader sees, when it must differ from the one on disk:
    /// some names cannot be created on every platform (Windows has no `"`).
    pub sent_name: Option<String>,
    pub content: Vec<u8>,
    pub credentials: Credentials,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            name: "sample.bin".into(),
            sent_name: None,
            content: b"0123456789abcdef".to_vec(),
            credentials: Credentials::default(),
        }
    }
}

pub fn creds(pairs: &[(&str, &str)]) -> Credentials {
    pairs.iter().copied().collect()
}

pub async fn run(provider: &dyn Provider, mock: &Mock, config: RunConfig) -> Run {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(&config.name);
    std::fs::write(&path, &config.content).unwrap();
    let mut file = LocalFile::open(&path).await.unwrap();
    if let Some(name) = &config.sent_name {
        file.name = name.clone();
    }

    let http = HttpFactory::new()
        .with_proxy(ProxyMode::Direct)
        .with_url_rewrite(mock.rewrite())
        .client()
        .unwrap();
    let progress = Progress::new(file.size);
    let cx = UploadContext {
        file: &file,
        http: &http,
        credentials: &config.credentials,
        progress: &progress,
    };
    let result = provider.upload(&cx).await;
    Run {
        result,
        progress,
        file,
    }
}

/// Shorthand: run with default content and no credentials.
pub async fn run_default(provider: &dyn Provider, mock: &Mock) -> Run {
    run(provider, mock, RunConfig::default()).await
}

pub fn builtin(name: &str) -> std::sync::Arc<dyn Provider> {
    mul_tor_core::Registry::builtin()
        .find(name)
        .unwrap_or_else(|| panic!("provider `{name}` is not registered"))
        .clone()
}

use std::path::PathBuf;

/// Why a single upload did not produce a link.
#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("file is larger than the {limit} this provider accepts")]
    TooLarge { limit: String },

    #[error("file is empty")]
    EmptyFile,

    #[error("missing credential `{0}` (set it in the config file)")]
    MissingCredential(&'static str),

    #[error("this provider does not accept the file: {0}")]
    Rejected(String),

    #[error("network error: {0}")]
    Network(reqwest::Error),

    #[error("could not read the file: {0}")]
    Io(#[from] std::io::Error),

    /// The provider answered, but not in the way the uploader expects. `raw` keeps
    /// a truncated copy of the response for the log.
    #[error("unexpected response from the provider: {message}")]
    Protocol {
        message: String,
        raw: Option<String>,
    },

    #[error("cancelled")]
    Cancelled,
}

impl UploadError {
    pub fn protocol(message: impl Into<String>, raw: impl AsRef<str>) -> Self {
        let raw = raw.as_ref();
        UploadError::Protocol {
            message: message.into(),
            raw: if raw.is_empty() {
                None
            } else {
                Some(truncate_for_log(raw))
            },
        }
    }

    /// Protocol error without a response body to attach.
    pub fn unexpected(message: impl Into<String>) -> Self {
        UploadError::Protocol {
            message: message.into(),
            raw: None,
        }
    }

    /// A short, stable identifier for the kind of failure (for machine-readable output).
    pub fn kind(&self) -> &'static str {
        match self {
            UploadError::TooLarge { .. } => "too_large",
            UploadError::EmptyFile => "empty_file",
            UploadError::MissingCredential(_) => "missing_credential",
            UploadError::Rejected(_) => "rejected",
            UploadError::Network(_) => "network",
            UploadError::Io(_) => "io",
            UploadError::Protocol { .. } => "protocol",
            UploadError::Cancelled => "cancelled",
        }
    }

    /// The raw response excerpt, if one was captured.
    pub fn raw(&self) -> Option<&str> {
        match self {
            UploadError::Protocol { raw, .. } => raw.as_deref(),
            _ => None,
        }
    }
}

/// Some APIs (Rapidgator) take credentials in the query string, and reqwest
/// prints the request URL in its error message. Every `reqwest::Error` that
/// enters one of our error types goes through here first.
fn without_secrets(mut err: reqwest::Error) -> reqwest::Error {
    if let Some(url) = err.url_mut() {
        url.set_query(None);
        url.set_fragment(None);
    }
    err
}

impl From<reqwest::Error> for UploadError {
    fn from(err: reqwest::Error) -> Self {
        UploadError::Network(without_secrets(err))
    }
}

fn truncate_for_log(raw: &str) -> String {
    const MAX: usize = 2000;
    if raw.len() <= MAX {
        return raw.to_string();
    }
    let mut end = MAX;
    while !raw.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… [truncated, {} bytes total]", &raw[..end], raw.len())
}

/// Errors from everything that is not an upload: config, presets, setup.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid config file {}: {message}", path.display())]
    Config { path: PathBuf, message: String },

    #[error("invalid preset `{name}`: {message}")]
    Preset { name: String, message: String },

    #[error(transparent)]
    Http(reqwest::Error),

    #[error("{0}")]
    Other(String),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Http(without_secrets(err))
    }
}

impl Error {
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Error::Io {
            context: context.into(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_excerpts_are_truncated_on_a_char_boundary() {
        let body = "é".repeat(5000);
        let err = UploadError::protocol("bad", &body);
        let raw = err.raw().unwrap();
        assert!(raw.len() < 2200);
        assert!(raw.contains("truncated"));
    }

    #[test]
    fn kinds_are_stable_identifiers() {
        assert_eq!(UploadError::EmptyFile.kind(), "empty_file");
        assert_eq!(UploadError::Cancelled.kind(), "cancelled");
        assert_eq!(UploadError::unexpected("x").kind(), "protocol");
        assert_eq!(
            UploadError::MissingCredential("k").kind(),
            "missing_credential"
        );
    }

    #[test]
    fn network_errors_do_not_leak_query_string_credentials() {
        // Build a reqwest error carrying a URL with a secret in the query.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        // Go through the factory like production code: it installs the TLS
        // provider, which a bare reqwest client would otherwise panic without.
        let http = crate::HttpFactory::new()
            .with_proxy(crate::ProxyMode::Direct)
            .client()
            .unwrap();
        let err = rt.block_on(async {
            http.get("http://127.0.0.1:1/login?password=hunter2")
                .send()
                .await
                .unwrap_err()
        });
        let shown = UploadError::from(err).to_string();
        assert!(!shown.contains("hunter2"), "{shown}");
    }

    #[test]
    fn setup_errors_do_not_leak_query_string_credentials_either() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let http = crate::HttpFactory::new()
            .with_proxy(crate::ProxyMode::Direct)
            .client()
            .unwrap();
        let err = rt.block_on(async {
            http.get("http://127.0.0.1:1/list?token=hunter2")
                .send()
                .await
                .unwrap_err()
        });
        let shown = Error::from(err).to_string();
        assert!(!shown.contains("hunter2"), "{shown}");
    }
}

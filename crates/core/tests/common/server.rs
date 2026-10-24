//! The in-process HTTP server.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderName, HeaderValue, Request, Response, StatusCode};
use mul_tor_core::http::UrlRewrite;

use super::recorded::Recorded;
use super::reply::Reply;

type Handler = dyn Fn(&Recorded) -> Reply + Send + Sync;

pub struct Mock {
    port: u16,
    requests: Arc<Mutex<Vec<Recorded>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Mock {
    pub async fn start(handler: impl Fn(&Recorded) -> Reply + Send + Sync + 'static) -> Mock {
        let requests: Arc<Mutex<Vec<Recorded>>> = Arc::default();
        let handler: Arc<Handler> = Arc::new(handler);
        let log = requests.clone();

        let app = Router::new().fallback(move |request: Request<Body>| {
            let handler = handler.clone();
            let log = log.clone();
            async move {
                let (parts, body) = request.into_parts();
                let body = to_bytes(body, usize::MAX).await.unwrap().to_vec();
                // `/host/the/real/path` -> host + path
                let full = parts.uri.path().trim_start_matches('/');
                let (host, path) = match full.split_once('/') {
                    Some((host, rest)) => (host.to_string(), format!("/{rest}")),
                    None => (full.to_string(), "/".to_string()),
                };
                let recorded = Recorded {
                    method: parts.method.to_string(),
                    host,
                    path,
                    query: parts.uri.query().unwrap_or("").to_string(),
                    headers: parts.headers,
                    body,
                };
                let reply = handler(&recorded);
                log.lock().unwrap().push(recorded);

                let mut response =
                    Response::builder().status(StatusCode::from_u16(reply.status).unwrap());
                for (name, value) in &reply.headers {
                    response = response.header(
                        HeaderName::from_bytes(name.as_bytes()).unwrap(),
                        HeaderValue::from_str(value).unwrap(),
                    );
                }
                response.body(Body::from(reply.body)).unwrap()
            }
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Mock {
            port,
            requests,
            task,
        }
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }

    /// Requests that were *not* the given host (e.g. to spot leaks).
    pub fn requests_to(&self, host: &str) -> Vec<Recorded> {
        self.requests()
            .into_iter()
            .filter(|r| r.host == host)
            .collect()
    }

    /// Maps `https://host/path?q` onto this server as `/host/path?q`.
    pub fn rewrite(&self) -> UrlRewrite {
        let port = self.port;
        Arc::new(move |url: &str| {
            let rest = url
                .strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))
                .unwrap_or(url);
            format!("http://127.0.0.1:{port}/{rest}")
        })
    }
}

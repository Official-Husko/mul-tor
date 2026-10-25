//! The in-process HTTP server.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::ws::rejection::WebSocketUpgradeRejection;
use axum::http::{HeaderName, HeaderValue, Request, Response, StatusCode};
use axum::response::IntoResponse;
use mul_tor_core::http::UrlRewrite;

use super::recorded::Recorded;
use super::reply::Reply;
use super::socket::{Frame, SocketHandler, SocketLog, serve_socket};

type Handler = dyn Fn(&Recorded) -> Reply + Send + Sync;

pub struct Mock {
    port: u16,
    requests: Arc<Mutex<Vec<Recorded>>>,
    socket_log: Arc<SocketLog>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Mock {
    pub async fn start(handler: impl Fn(&Recorded) -> Reply + Send + Sync + 'static) -> Mock {
        Self::serve(handler, None).await
    }

    /// Like [`Mock::start`], and a WebSocket upgrade is accepted: `on_text` gets
    /// every text message and returns the messages to send back.
    pub async fn start_with_socket(
        handler: impl Fn(&Recorded) -> Reply + Send + Sync + 'static,
        on_text: impl Fn(&str) -> Vec<String> + Send + Sync + 'static,
    ) -> Mock {
        Self::start_with_frames(handler, move |frame| match frame {
            Frame::Text(text) => on_text(text),
            Frame::Binary(_) => Vec::new(),
        })
        .await
    }

    /// Like [`Mock::start_with_socket`], for sockets that also receive binary frames.
    pub async fn start_with_frames(
        handler: impl Fn(&Recorded) -> Reply + Send + Sync + 'static,
        on_frame: impl Fn(Frame<'_>) -> Vec<String> + Send + Sync + 'static,
    ) -> Mock {
        Self::serve(handler, Some(Arc::new(on_frame))).await
    }

    async fn serve(
        handler: impl Fn(&Recorded) -> Reply + Send + Sync + 'static,
        socket: Option<Arc<SocketHandler>>,
    ) -> Mock {
        let requests: Arc<Mutex<Vec<Recorded>>> = Arc::default();
        let socket_log: Arc<SocketLog> = Arc::default();
        let handler: Arc<Handler> = Arc::new(handler);
        let log = requests.clone();
        let received = socket_log.clone();

        let app = Router::new().fallback(
            move |ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
                  request: Request<Body>| {
                let handler = handler.clone();
                let log = log.clone();
                let socket = socket.clone();
                let received = received.clone();
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
                    if let (Ok(upgrade), Some(socket)) = (ws, socket) {
                        log.lock().unwrap().push(recorded);
                        return upgrade
                            .on_upgrade(move |connection| {
                                serve_socket(connection, socket, received)
                            })
                            .into_response();
                    }
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
                    response
                        .body(Body::from(reply.body))
                        .unwrap()
                        .into_response()
                }
            },
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Mock {
            port,
            requests,
            socket_log,
            task,
        }
    }

    /// Text messages that arrived over a WebSocket, in order.
    pub fn messages(&self) -> Vec<String> {
        self.socket_log.text.lock().unwrap().clone()
    }

    /// Binary frames that arrived over a WebSocket, in order.
    pub fn binary_frames(&self) -> Vec<Vec<u8>> {
        self.socket_log.binary.lock().unwrap().clone()
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

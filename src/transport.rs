//! The HTTP and timing seams the client is built on.
//!
//! Most callers never touch this module: [`Client`](crate::Client) wires up a
//! [`reqwest`]-backed transport and a real clock automatically. It is public so
//! that advanced callers can plug in a custom HTTP stack (an edge runtime, an
//! instrumented client) or, in tests, a deterministic fake.

use std::time::Duration;

use async_trait::async_trait;

/// HTTP method for a [`HttpRequest`]. Reqwest-agnostic on purpose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Patch,
    Delete,
}

/// A fully-formed request handed to a [`Transport`]. The `url` already carries
/// the base URL, path, and query string; headers and body are final.
#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

/// A raw HTTP response. The body is the undecoded bytes.
#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// A transport-level failure (no HTTP response was produced). The client maps
/// this to [`Error::Connection`](crate::Error::Connection).
#[derive(Clone, Debug)]
pub struct TransportError {
    pub message: String,
    pub timeout: bool,
}

/// Performs a single HTTP round-trip. Implement this to swap in a custom HTTP
/// stack; the default is [`reqwest`].
#[async_trait]
pub trait Transport: Send + Sync {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, TransportError>;
}

/// The clock the retry loop sleeps against, and the jitter source for backoff.
/// The default sleeps on the Tokio timer; tests inject a no-op.
#[async_trait]
pub trait Sleeper: Send + Sync {
    async fn sleep(&self, duration: Duration);
    /// A value in `[0, 1)` used as the full-jitter factor for one backoff.
    fn jitter(&self) -> f64;
}

/// The default [`Transport`], backed by a `reqwest::Client`.
pub(crate) struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub(crate) fn new(timeout: Duration) -> Result<Self, TransportError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| TransportError {
                message: e.to_string(),
                timeout: false,
            })?;
        Ok(Self { client })
    }
}

#[async_trait]
impl Transport for ReqwestTransport {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        let method = match request.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Patch => reqwest::Method::PATCH,
            Method::Delete => reqwest::Method::DELETE,
        };

        let mut builder = self.client.request(method, &request.url);
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if let Some(body) = request.body {
            builder = builder.body(body);
        }

        let response = builder.send().await.map_err(|e| TransportError {
            message: e.to_string(),
            timeout: e.is_timeout(),
        })?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    value.to_str().unwrap_or("").to_string(),
                )
            })
            .collect();
        let body = response
            .bytes()
            .await
            .map_err(|e| TransportError {
                message: e.to_string(),
                timeout: e.is_timeout(),
            })?
            .to_vec();

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

/// The default [`Sleeper`]: the Tokio timer plus `fastrand` jitter.
pub(crate) struct RealSleeper;

#[async_trait]
impl Sleeper for RealSleeper {
    async fn sleep(&self, duration: Duration) {
        if !duration.is_zero() {
            tokio::time::sleep(duration).await;
        }
    }

    fn jitter(&self) -> f64 {
        fastrand::f64()
    }
}

use std::sync::Arc;

use crate::error::Result;
use crate::http::HttpClient;
use crate::resources::to_value;
use crate::response::Response;
use crate::transport::Method;
use crate::types::email::{BatchEmail, SendEmail};

/// Operations on the `/email` endpoints.
///
/// Attachment `content` is raw bytes; the SDK base64-encodes it for transport.
pub struct Email {
    http: Arc<HttpClient>,
}

impl Email {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// Send a single message. Returns the queued message; raises an
    /// [`Error`](crate::Error) on failure.
    ///
    /// With retries enabled (the default) and no explicit key, the client
    /// generates an idempotency key and reuses it across retries so a retried
    /// send cannot deliver twice.
    pub async fn send(&self, email: &SendEmail) -> Result<Response> {
        self.http
            .request_object(Method::Post, "/email", Some(to_value(email)?), true, None)
            .await
    }

    /// Send a single message with an explicit idempotency key, to dedupe across
    /// process restarts.
    pub async fn send_with_idempotency_key(
        &self,
        email: &SendEmail,
        key: &str,
    ) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                "/email",
                Some(to_value(email)?),
                true,
                Some(key),
            )
            .await
    }

    /// Send 1–100 independent messages in one request.
    ///
    /// A mixed-outcome batch (HTTP 207) returns normally — inspect each entry's
    /// `status` in `data`; it does not raise.
    pub async fn send_batch(&self, batch: &BatchEmail) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                "/email/batch",
                Some(to_value(batch)?),
                true,
                None,
            )
            .await
    }

    /// Send a batch with an explicit idempotency key.
    pub async fn send_batch_with_idempotency_key(
        &self,
        batch: &BatchEmail,
        key: &str,
    ) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                "/email/batch",
                Some(to_value(batch)?),
                true,
                Some(key),
            )
            .await
    }
}

use std::sync::Arc;

use serde::Serialize;

use crate::error::Result;
use crate::http::HttpClient;
use crate::resources::{enc, to_value};
use crate::response::{Page, Response};
use crate::transport::Method;
use crate::types::params::ListParams;

/// Operations on the `/webhooks` endpoints.
pub struct Webhooks {
    http: Arc<HttpClient>,
}

impl Webhooks {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// List the team's webhooks, newest-first.
    pub async fn list(&self, params: ListParams) -> Result<Page> {
        self.http
            .list(
                "/webhooks",
                vec![
                    ("limit", params.limit.map(|n| n.to_string())),
                    ("after", params.after),
                ],
            )
            .await
    }

    /// List every webhook, walking all pages.
    pub async fn list_all(&self, params: ListParams) -> Result<Vec<Response>> {
        self.http
            .list_all(
                "/webhooks",
                vec![("limit", params.limit.map(|n| n.to_string()))],
            )
            .await
    }

    /// Create a webhook. The full `signing_secret` is on this response only —
    /// store it now to verify future deliveries; later reads return only the
    /// prefix.
    pub async fn create(&self, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                "/webhooks",
                Some(to_value(body)?),
                false,
                None,
            )
            .await
    }

    /// Retrieve a webhook. The signing secret is never returned — only its prefix.
    pub async fn get(&self, id: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Get,
                &format!("/webhooks/{}", enc(id)),
                None,
                false,
                None,
            )
            .await
    }

    /// Update a webhook's name, URL, subscribed events, and status. Does not
    /// rotate the signing secret — use [`Webhooks::rotate_secret`].
    pub async fn update(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(
                Method::Patch,
                &format!("/webhooks/{}", enc(id)),
                Some(to_value(body)?),
                false,
                None,
            )
            .await
    }

    /// Permanently delete a webhook.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.http
            .request_empty(Method::Delete, &format!("/webhooks/{}", enc(id)))
            .await
    }

    /// Send one synthetic `webhook.test` event and report the outcome. Returns
    /// the result even when the endpoint fails — read `delivered` and
    /// `status_code`.
    pub async fn test(&self, id: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                &format!("/webhooks/{}/test", enc(id)),
                None,
                false,
                None,
            )
            .await
    }

    /// Rotate the signing secret. The new secret is on this response only; the
    /// previous secret stays valid for a 24h grace window. Rotating again before
    /// the window ends raises `webhook_rotation_in_progress`
    /// ([`Error::Conflict`](crate::Error::Conflict)).
    pub async fn rotate_secret(&self, id: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                &format!("/webhooks/{}/rotate-secret", enc(id)),
                None,
                false,
                None,
            )
            .await
    }
}

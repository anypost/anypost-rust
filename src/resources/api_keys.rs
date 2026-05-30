use std::sync::Arc;

use serde::Serialize;

use crate::error::Result;
use crate::http::HttpClient;
use crate::resources::{enc, to_value};
use crate::response::{Page, Response};
use crate::transport::Method;
use crate::types::params::ListParams;

/// Operations on the `/api-keys` endpoints.
pub struct ApiKeys {
    http: Arc<HttpClient>,
}

impl ApiKeys {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// List the team's API keys, newest-first.
    pub async fn list(&self, params: ListParams) -> Result<Page> {
        self.http
            .list(
                "/api-keys",
                vec![
                    ("limit", params.limit.map(|n| n.to_string())),
                    ("after", params.after),
                ],
            )
            .await
    }

    /// List every API key, walking all pages.
    pub async fn list_all(&self, params: ListParams) -> Result<Vec<Response>> {
        self.http
            .list_all(
                "/api-keys",
                vec![("limit", params.limit.map(|n| n.to_string()))],
            )
            .await
    }

    /// Issue a new API key. The plaintext secret is returned only here, as
    /// `key` — store it securely; it cannot be retrieved later.
    pub async fn create(&self, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                "/api-keys",
                Some(to_value(body)?),
                false,
                None,
            )
            .await
    }

    /// Retrieve a single API key's metadata. The secret is never returned.
    pub async fn get(&self, id: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Get,
                &format!("/api-keys/{}", enc(id)),
                None,
                false,
                None,
            )
            .await
    }

    /// Update a key's name, permissions, and restrictions. Changes may take up
    /// to 5 minutes to propagate.
    pub async fn update(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(
                Method::Patch,
                &format!("/api-keys/{}", enc(id)),
                Some(to_value(body)?),
                false,
                None,
            )
            .await
    }

    /// Delete a key. It may keep authenticating for up to 5 minutes (gateway cache).
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.http
            .request_empty(Method::Delete, &format!("/api-keys/{}", enc(id)))
            .await
    }
}

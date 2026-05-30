use std::sync::Arc;

use serde::Serialize;

use crate::error::Result;
use crate::http::HttpClient;
use crate::resources::{enc, to_value};
use crate::response::{Page, Response};
use crate::transport::Method;
use crate::types::params::ListParams;

/// Operations on the `/domains` endpoints.
pub struct Domains {
    http: Arc<HttpClient>,
}

impl Domains {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// List the team's domains, newest-first. Returns one page.
    pub async fn list(&self, params: ListParams) -> Result<Page> {
        self.http
            .list(
                "/domains",
                vec![
                    ("limit", params.limit.map(|n| n.to_string())),
                    ("after", params.after),
                ],
            )
            .await
    }

    /// List every domain, walking all pages.
    pub async fn list_all(&self, params: ListParams) -> Result<Vec<Response>> {
        self.http
            .list_all(
                "/domains",
                vec![("limit", params.limit.map(|n| n.to_string()))],
            )
            .await
    }

    /// Add a sending domain. The returned domain is `pending` until verified.
    pub async fn create(&self, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(Method::Post, "/domains", Some(to_value(body)?), false, None)
            .await
    }

    /// Retrieve a single domain by id.
    pub async fn get(&self, id: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Get,
                &format!("/domains/{}", enc(id)),
                None,
                false,
                None,
            )
            .await
    }

    /// Update a domain's tracking configuration. The domain `name` is immutable.
    pub async fn update(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(
                Method::Patch,
                &format!("/domains/{}", enc(id)),
                Some(to_value(body)?),
                false,
                None,
            )
            .await
    }

    /// Permanently delete a domain and its DKIM keys.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.http
            .request_empty(Method::Delete, &format!("/domains/{}", enc(id)))
            .await
    }

    /// Trigger a verification check. Always returns the current domain — read
    /// `status` and `verification_failure`; a still-`pending` domain does not
    /// raise. Safe to poll while DNS propagates.
    pub async fn verify(&self, id: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                &format!("/domains/{}/verify", enc(id)),
                None,
                false,
                None,
            )
            .await
    }
}

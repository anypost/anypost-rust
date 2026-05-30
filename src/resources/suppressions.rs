use std::sync::Arc;

use serde::Serialize;

use crate::error::Result;
use crate::http::HttpClient;
use crate::resources::{enc, to_value};
use crate::response::{Page, Response};
use crate::transport::Method;
use crate::types::params::SuppressionListParams;

/// Operations on the `/suppressions` endpoints. Entries key on `(email, topic)`.
pub struct Suppressions {
    http: Arc<HttpClient>,
}

impl Suppressions {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// List the team's suppressions, newest-first. Expired rows are filtered out.
    pub async fn list(&self, params: SuppressionListParams) -> Result<Page> {
        self.http.list("/suppressions", Self::query(params)).await
    }

    /// List every suppression, walking all pages.
    pub async fn list_all(&self, params: SuppressionListParams) -> Result<Vec<Response>> {
        let mut query = Self::query(params);
        query.retain(|(key, _)| *key != "after");
        self.http.list_all("/suppressions", query).await
    }

    /// Add a manual suppression. Defaults to topic `*` (every topic). Raises
    /// `validation_error` if an active entry for the same `(email, topic)` exists.
    pub async fn create(&self, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                "/suppressions",
                Some(to_value(body)?),
                false,
                None,
            )
            .await
    }

    /// Retrieve the suppression for an `(email, topic)` pair. Use `*` as the
    /// topic for the global row.
    pub async fn get(&self, email: &str, topic: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Get,
                &format!("/suppressions/{}/{}", enc(email), enc(topic)),
                None,
                false,
                None,
            )
            .await
    }

    /// Remove the single `(email, topic)` row. Other topics are untouched.
    pub async fn delete(&self, email: &str, topic: &str) -> Result<()> {
        self.http
            .request_empty(
                Method::Delete,
                &format!("/suppressions/{}/{}", enc(email), enc(topic)),
            )
            .await
    }

    /// List every suppression on file for an address, across all topics. Raises
    /// `not_found` if the address has no active suppressions.
    pub async fn list_for_email(&self, email: &str) -> Result<Vec<Response>> {
        self.http
            .request_data(Method::Get, &format!("/suppressions/{}", enc(email)))
            .await
    }

    /// Remove an address from the suppression list across every topic.
    pub async fn delete_for_email(&self, email: &str) -> Result<()> {
        self.http
            .request_empty(Method::Delete, &format!("/suppressions/{}", enc(email)))
            .await
    }

    fn query(params: SuppressionListParams) -> Vec<(&'static str, Option<String>)> {
        vec![
            ("limit", params.limit.map(|n| n.to_string())),
            ("after", params.after),
            ("email_contains", params.email_contains),
            ("topic", params.topic),
            ("reason", params.reason),
            ("origin", params.origin),
        ]
    }
}

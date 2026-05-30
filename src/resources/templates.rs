use std::sync::Arc;

use serde::Serialize;

use crate::error::Result;
use crate::http::HttpClient;
use crate::resources::{enc, to_value};
use crate::response::{Page, Response};
use crate::transport::Method;
use crate::types::params::ListParams;

/// Operations on the `/templates` endpoints, including the draft/publish flow.
pub struct Templates {
    http: Arc<HttpClient>,
}

impl Templates {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// List the team's templates, newest-first.
    pub async fn list(&self, params: ListParams) -> Result<Page> {
        self.http
            .list(
                "/templates",
                vec![
                    ("limit", params.limit.map(|n| n.to_string())),
                    ("after", params.after),
                ],
            )
            .await
    }

    /// List every template, walking all pages.
    pub async fn list_all(&self, params: ListParams) -> Result<Vec<Response>> {
        self.http
            .list_all(
                "/templates",
                vec![("limit", params.limit.map(|n| n.to_string()))],
            )
            .await
    }

    /// Create a template. It starts unpublished — publish it before sending.
    pub async fn create(&self, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                "/templates",
                Some(to_value(body)?),
                false,
                None,
            )
            .await
    }

    /// Retrieve a template, including its published content.
    pub async fn get(&self, id: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Get,
                &format!("/templates/{}", enc(id)),
                None,
                false,
                None,
            )
            .await
    }

    /// Update a template's `name`. Body content lives on the draft.
    pub async fn update(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(
                Method::Patch,
                &format!("/templates/{}", enc(id)),
                Some(to_value(body)?),
                false,
                None,
            )
            .await
    }

    /// Permanently delete a template.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.http
            .request_empty(Method::Delete, &format!("/templates/{}", enc(id)))
            .await
    }

    /// Copy a template. The copy starts unpublished with a draft seeded from the
    /// source's current editable content.
    pub async fn duplicate(&self, id: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                &format!("/templates/{}/duplicate", enc(id)),
                None,
                false,
                None,
            )
            .await
    }

    /// Copy a template, overriding fields (e.g. `name`) on the copy.
    pub async fn duplicate_with(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                &format!("/templates/{}/duplicate", enc(id)),
                Some(to_value(body)?),
                false,
                None,
            )
            .await
    }

    /// Retrieve the template's unpublished draft. Raises `not_found` if none exists.
    pub async fn get_draft(&self, id: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Get,
                &format!("/templates/{}/draft", enc(id)),
                None,
                false,
                None,
            )
            .await
    }

    /// Create or update the template's draft. Idempotent upsert; published
    /// content untouched.
    pub async fn update_draft(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.http
            .request_object(
                Method::Patch,
                &format!("/templates/{}/draft", enc(id)),
                Some(to_value(body)?),
                false,
                None,
            )
            .await
    }

    /// Discard the template's draft without touching published content.
    pub async fn delete_draft(&self, id: &str) -> Result<()> {
        self.http
            .request_empty(Method::Delete, &format!("/templates/{}/draft", enc(id)))
            .await
    }

    /// Promote the draft into the published slot, consuming the draft.
    pub async fn publish(&self, id: &str) -> Result<Response> {
        self.http
            .request_object(
                Method::Post,
                &format!("/templates/{}/publish", enc(id)),
                None,
                false,
                None,
            )
            .await
    }
}

use std::sync::Arc;

use crate::error::Result;
use crate::http::HttpClient;
use crate::response::{Page, Response};
use crate::types::params::EventListParams;

/// Read access to the `/events` stream. List-only — events are not addressable
/// by id.
pub struct Events {
    http: Arc<HttpClient>,
}

impl Events {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// Page through the team's events, newest-first.
    pub async fn list(&self, params: EventListParams) -> Result<Page> {
        self.http.list("/events", Self::query(params)).await
    }

    /// List every matching event, walking all pages.
    pub async fn list_all(&self, params: EventListParams) -> Result<Vec<Response>> {
        let mut query = Self::query(params);
        query.retain(|(key, _)| *key != "after");
        self.http.list_all("/events", query).await
    }

    fn query(params: EventListParams) -> Vec<(&'static str, Option<String>)> {
        let tags = if params.tags.is_empty() {
            None
        } else {
            // Sent comma-separated (tags=a,b); the API matches with hasAny.
            Some(params.tags.join(","))
        };
        vec![
            ("limit", params.limit.map(|n| n.to_string())),
            ("after", params.after),
            ("start", params.start),
            ("end", params.end),
            ("event_type", params.event_type),
            ("recipient", params.recipient),
            ("email_id", params.email_id),
            ("message_id", params.message_id),
            ("domain", params.domain),
            ("topic", params.topic),
            ("campaign", params.campaign),
            ("template_id", params.template_id),
            ("tags", tags),
        ]
    }
}

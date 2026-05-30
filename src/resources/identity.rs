use std::sync::Arc;

use crate::error::Result;
use crate::http::HttpClient;
use crate::response::Response;
use crate::transport::Method;

/// Identity operations (`/whoami`).
pub struct Identity {
    http: Arc<HttpClient>,
}

impl Identity {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// Identify the team and permission level behind the current API key.
    pub async fn whoami(&self) -> Result<Response> {
        self.http
            .request_object(Method::Get, "/whoami", None, false, None)
            .await
    }
}

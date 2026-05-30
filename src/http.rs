//! The transport-agnostic request loop: header assembly, retries with
//! full-jitter backoff, idempotency keys, and error mapping.

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::error::{self, Error, Result};
use crate::response::{Page, Response};
use crate::transport::{HttpRequest, Method, Sleeper, Transport};
use crate::version::VERSION;

const RETRYABLE_STATUS: [u16; 3] = [429, 502, 503];
const MAX_BACKOFF: Duration = Duration::from_secs(8);
const BASE_BACKOFF_MS: u64 = 500;

pub(crate) struct HttpClient {
    transport: Arc<dyn Transport>,
    sleeper: Arc<dyn Sleeper>,
    base_url: String,
    api_key: String,
    default_headers: Vec<(String, String)>,
    max_retries: u32,
}

impl HttpClient {
    pub(crate) fn new(
        transport: Arc<dyn Transport>,
        sleeper: Arc<dyn Sleeper>,
        base_url: String,
        api_key: String,
        default_headers: Vec<(String, String)>,
        max_retries: u32,
    ) -> Self {
        Self {
            transport,
            sleeper,
            base_url,
            api_key,
            default_headers,
            max_retries,
        }
    }

    /// Perform a request and return the decoded JSON body (`Value::Null` on 204
    /// or an empty body).
    async fn request(
        &self,
        method: Method,
        path: &str,
        query: Vec<(String, String)>,
        body: Option<Vec<u8>>,
        idempotent: bool,
        idempotency_key: Option<&str>,
    ) -> Result<Value> {
        let url = self.build_url(path, &query)?;
        let headers = self.build_headers(body.is_some(), idempotent, idempotency_key);

        let mut attempt: u32 = 0;
        loop {
            let request = HttpRequest {
                method,
                url: url.clone(),
                headers: headers.clone(),
                body: body.clone(),
            };

            match self.transport.execute(request).await {
                Err(err) => {
                    if attempt < self.max_retries {
                        self.backoff(attempt, None).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(Error::connection(err));
                }
                Ok(response) => {
                    let status = response.status;
                    if (200..300).contains(&status) {
                        return Ok(decode_body(&response.body));
                    }
                    if RETRYABLE_STATUS.contains(&status) && attempt < self.max_retries {
                        self.backoff(attempt, Some(&response.headers)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(Error::from_response(
                        status,
                        &response.body,
                        &response.headers,
                    ));
                }
            }
        }
    }

    pub(crate) async fn request_object(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        idempotent: bool,
        idempotency_key: Option<&str>,
    ) -> Result<Response> {
        let bytes = encode_body(body)?;
        let value = self
            .request(method, path, Vec::new(), bytes, idempotent, idempotency_key)
            .await?;
        Ok(Response::new(ensure_object(value)))
    }

    pub(crate) async fn request_empty(&self, method: Method, path: &str) -> Result<()> {
        self.request(method, path, Vec::new(), None, false, None)
            .await?;
        Ok(())
    }

    /// GET a list endpoint and return one page.
    pub(crate) async fn list(
        &self,
        path: &str,
        query: Vec<(&str, Option<String>)>,
    ) -> Result<Page> {
        let value = self
            .request(Method::Get, path, clean_query(query), None, false, None)
            .await?;
        Ok(Page::from_value(value))
    }

    /// Walk every page of a list endpoint, following `next_cursor`.
    pub(crate) async fn list_all(
        &self,
        path: &str,
        base_query: Vec<(&str, Option<String>)>,
    ) -> Result<Vec<Response>> {
        let mut items = Vec::new();
        let mut after: Option<String> = None;
        loop {
            let mut query = base_query.clone();
            if let Some(cursor) = &after {
                query.push(("after", Some(cursor.clone())));
            }
            let page = self.list(path, query).await?;
            items.extend(page.data);
            match (page.has_more, page.next_cursor) {
                (true, Some(cursor)) => after = Some(cursor),
                _ => break,
            }
        }
        Ok(items)
    }

    /// GET an endpoint that returns `{ "data": [...] }` and collect the rows.
    pub(crate) async fn request_data(&self, method: Method, path: &str) -> Result<Vec<Response>> {
        let value = self
            .request(method, path, Vec::new(), None, false, None)
            .await?;
        Ok(value
            .get("data")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().cloned().map(Response::new).collect())
            .unwrap_or_default())
    }

    fn build_url(&self, path: &str, query: &[(String, String)]) -> Result<String> {
        let full = format!("{}{}", self.base_url, path);
        let mut url = reqwest::Url::parse(&full)
            .map_err(|e| Error::Serialization(format!("could not build request URL: {e}")))?;
        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }
        Ok(url.to_string())
    }

    fn build_headers(
        &self,
        has_body: bool,
        idempotent: bool,
        idempotency_key: Option<&str>,
    ) -> Vec<(String, String)> {
        let mut headers = vec![
            (
                "Authorization".to_string(),
                format!("Bearer {}", self.api_key),
            ),
            ("Accept".to_string(), "application/json".to_string()),
            ("User-Agent".to_string(), format!("anypost-rust/{VERSION}")),
        ];

        for (key, value) in &self.default_headers {
            headers.push((key.clone(), value.clone()));
        }

        if has_body {
            headers.push(("Content-Type".to_string(), "application/json".to_string()));
        }

        if idempotent {
            match idempotency_key {
                Some(key) if !key.is_empty() => {
                    headers.push(("Idempotency-Key".to_string(), key.to_string()));
                }
                // Auto-key so built-in retries of a send cannot deliver twice.
                _ if self.max_retries > 0 => {
                    headers.push(("Idempotency-Key".to_string(), uuid_v4()));
                }
                _ => {}
            }
        }

        headers
    }

    async fn backoff(&self, attempt: u32, headers: Option<&[(String, String)]>) {
        if let Some(headers) = headers {
            if let Some(seconds) = error::retry_after_seconds(headers) {
                let capped = seconds.min(MAX_BACKOFF.as_secs_f64());
                self.sleeper.sleep(Duration::from_secs_f64(capped)).await;
                return;
            }
        }

        let ceiling_ms = BASE_BACKOFF_MS
            .saturating_mul(1u64 << attempt.min(20))
            .min(MAX_BACKOFF.as_millis() as u64);
        let jitter = self.sleeper.jitter().clamp(0.0, 1.0); // full jitter
        let delay = Duration::from_millis((ceiling_ms as f64 * jitter) as u64);
        self.sleeper.sleep(delay).await;
    }
}

fn encode_body(body: Option<Value>) -> Result<Option<Vec<u8>>> {
    match body {
        None => Ok(None),
        Some(value) => serde_json::to_vec(&value)
            .map(Some)
            .map_err(|e| Error::Serialization(e.to_string())),
    }
}

fn decode_body(body: &[u8]) -> Value {
    if body.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(body).unwrap_or(Value::Null)
}

fn ensure_object(value: Value) -> Value {
    if value.is_object() {
        value
    } else {
        Value::Object(serde_json::Map::new())
    }
}

/// A random v4 UUID for an auto idempotency key. Uniqueness, not cryptographic
/// strength, is what matters here — the key only deduplicates a retried send.
fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    for byte in bytes.iter_mut() {
        *byte = fastrand::u8(..);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 1
    let h = hex::encode(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

fn clean_query(query: Vec<(&str, Option<String>)>) -> Vec<(String, String)> {
    query
        .into_iter()
        .filter_map(|(key, value)| value.map(|v| (key.to_string(), v)))
        .collect()
}

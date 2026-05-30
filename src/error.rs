//! Error types for the SDK.
//!
//! A failed request resolves to an [`Error`]. Match on the variant (which
//! corresponds to the stable, machine-readable `error.type`) rather than on the
//! HTTP status. [`Error::error_type`] exposes the raw code when a single
//! variant covers several types (e.g. [`Error::Conflict`]).

use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::transport::TransportError;

/// A convenience alias for results returned by this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// The payload carried by every API-level error: the canonical envelope plus
/// the HTTP status and request id.
#[derive(Clone, Debug)]
pub struct ApiError {
    /// The stable, machine-readable `error.type`.
    pub r#type: String,
    /// A single human-readable sentence.
    pub message: String,
    /// The HTTP status, when a response was received.
    pub status: Option<u16>,
    /// The request id from the response, when present.
    pub request_id: Option<String>,
    /// Field-level problems. Populated only for `validation_error`.
    pub errors: BTreeMap<String, Vec<String>>,
    /// Parsed `Retry-After`, in seconds. Populated only for `rate_limit_exceeded`.
    pub retry_after: Option<f64>,
    /// The parsed response body.
    pub raw: Option<Value>,
}

/// Every error this crate can return.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// `validation_error` (400/422). See [`ApiError::errors`].
    Validation(ApiError),
    /// `authentication_error` (401).
    Authentication(ApiError),
    /// `permission_error` (403).
    Permission(ApiError),
    /// `not_found` (404).
    NotFound(ApiError),
    /// `conflict`, `idempotency_concurrent`, or `webhook_rotation_in_progress` (409).
    Conflict(ApiError),
    /// `idempotency_mismatch` (422).
    IdempotencyMismatch(ApiError),
    /// `rate_limit_exceeded` (429). See [`ApiError::retry_after`].
    RateLimit(ApiError),
    /// `payload_too_large` (413).
    PayloadTooLarge(ApiError),
    /// A server error (5xx), including `internal_error` and `provisioning_error`.
    Api(ApiError),
    /// No HTTP response was received (network failure, timeout, or abort).
    Connection { message: String },
    /// A request body could not be serialized.
    Serialization(String),
    /// The client was misconfigured (e.g. no API key).
    Config(String),
}

impl Error {
    /// The underlying [`ApiError`] for the HTTP-level variants.
    pub fn api(&self) -> Option<&ApiError> {
        match self {
            Error::Validation(e)
            | Error::Authentication(e)
            | Error::Permission(e)
            | Error::NotFound(e)
            | Error::Conflict(e)
            | Error::IdempotencyMismatch(e)
            | Error::RateLimit(e)
            | Error::PayloadTooLarge(e)
            | Error::Api(e) => Some(e),
            _ => None,
        }
    }

    /// The stable, machine-readable error type. Branch on this when one variant
    /// covers several codes.
    pub fn error_type(&self) -> Option<&str> {
        match self {
            Error::Connection { .. } => Some("connection_error"),
            _ => self.api().map(|e| e.r#type.as_str()),
        }
    }

    /// The HTTP status, when there was a response.
    pub fn status(&self) -> Option<u16> {
        self.api().and_then(|e| e.status)
    }

    /// The request id from the response, when present.
    pub fn request_id(&self) -> Option<&str> {
        self.api().and_then(|e| e.request_id.as_deref())
    }

    /// Field-level validation problems, for [`Error::Validation`].
    pub fn validation_errors(&self) -> Option<&BTreeMap<String, Vec<String>>> {
        match self {
            Error::Validation(e) => Some(&e.errors),
            _ => None,
        }
    }

    /// The parsed `Retry-After`, in seconds, for [`Error::RateLimit`].
    pub fn retry_after(&self) -> Option<f64> {
        match self {
            Error::RateLimit(e) => e.retry_after,
            _ => None,
        }
    }

    pub(crate) fn connection(err: TransportError) -> Self {
        Error::Connection {
            message: format!("Could not reach Anypost: {}", err.message),
        }
    }

    /// Map an error response into the right variant. Keys primarily on the
    /// canonical `error.type`, falling back to the HTTP status.
    pub(crate) fn from_response(status: u16, body: &[u8], headers: &[(String, String)]) -> Self {
        let value: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
        let request_id = read_request_id(headers);

        let mut type_: Option<String> = None;
        let mut message: Option<String> = None;
        let mut errors: BTreeMap<String, Vec<String>> = BTreeMap::new();

        match value.get("error") {
            // Canonical envelope: { error: { type, message, errors? } }.
            Some(Value::Object(obj)) => {
                type_ = obj.get("type").and_then(Value::as_str).map(str::to_string);
                message = obj
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if let Some(Value::Object(map)) = obj.get("errors") {
                    for (key, val) in map {
                        if let Value::Array(items) = val {
                            errors.insert(
                                key.clone(),
                                items
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(str::to_string)
                                    .collect(),
                            );
                        }
                    }
                }
            }
            // Flat envelope: { error: "<code>", message? }.
            Some(Value::String(code)) => {
                type_ = Some(code.clone());
                message = value
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| Some(code.replace('_', " ")));
            }
            _ => {}
        }

        let type_ = type_.unwrap_or_else(|| type_from_status(status).to_string());
        let message = message.unwrap_or_else(|| default_message(status));
        let retry_after = retry_after_seconds(headers);

        let api = ApiError {
            r#type: type_,
            message,
            status: Some(status),
            request_id,
            errors,
            retry_after,
            raw: Some(value),
        };

        build(status, api)
    }
}

fn build(status: u16, api: ApiError) -> Error {
    match api.r#type.as_str() {
        "validation_error" => Error::Validation(api),
        "authentication_error" => Error::Authentication(api),
        "permission_error" => Error::Permission(api),
        "not_found" => Error::NotFound(api),
        "conflict" | "idempotency_concurrent" | "webhook_rotation_in_progress" => {
            Error::Conflict(api)
        }
        "idempotency_mismatch" => Error::IdempotencyMismatch(api),
        "rate_limit_exceeded" => Error::RateLimit(api),
        "payload_too_large" => Error::PayloadTooLarge(api),
        "provisioning_error" | "internal_error" => Error::Api(api),
        _ => by_status(status, api),
    }
}

fn by_status(status: u16, api: ApiError) -> Error {
    match status {
        401 => Error::Authentication(api),
        403 => Error::Permission(api),
        404 => Error::NotFound(api),
        409 => Error::Conflict(api),
        413 => Error::PayloadTooLarge(api),
        429 => Error::RateLimit(api),
        400 | 422 => Error::Validation(api),
        _ => Error::Api(api),
    }
}

fn type_from_status(status: u16) -> &'static str {
    match status {
        400 | 422 => "validation_error",
        401 => "authentication_error",
        403 => "permission_error",
        404 => "not_found",
        409 => "conflict",
        413 => "payload_too_large",
        429 => "rate_limit_exceeded",
        s if s >= 500 => "internal_error",
        _ => "api_error",
    }
}

fn default_message(status: u16) -> String {
    format!("Anypost request failed with status {status}.")
}

/// Parse a `Retry-After` header (delta-seconds or HTTP-date) into seconds.
pub(crate) fn retry_after_seconds(headers: &[(String, String)]) -> Option<f64> {
    let value = header(headers, "retry-after")?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(seconds) = value.parse::<f64>() {
        return Some(seconds.max(0.0));
    }
    if let Ok(when) = httpdate::parse_http_date(value) {
        let delta = when
            .duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO);
        return Some(delta.as_secs_f64());
    }
    None
}

const REQUEST_ID_HEADERS: [&str; 3] =
    ["anypost-request-id", "x-anypost-request-id", "x-request-id"];

pub(crate) fn read_request_id(headers: &[(String, String)]) -> Option<String> {
    for name in REQUEST_ID_HEADERS {
        if let Some(value) = header(headers, name) {
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Case-insensitive single-value header lookup.
fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Connection { message } => write!(f, "{message}"),
            Error::Serialization(message) => {
                write!(f, "could not serialize request body: {message}")
            }
            Error::Config(message) => write!(f, "{message}"),
            _ => {
                let api = self.api().expect("HTTP variants carry an ApiError");
                write!(f, "{} ({})", api.message, api.r#type)
            }
        }
    }
}

impl std::error::Error for Error {}

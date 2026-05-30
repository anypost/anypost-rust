//! Verify the signature on an Anypost webhook delivery.
//!
//! These are free functions — they need the webhook's signing secret, not an
//! API key, so call them in your handler without a [`Client`](crate::Client).
//! Pass the **raw** request body (the exact bytes received, before JSON
//! parsing), the `Anypost-Signature` header value, and the signing secret.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::response::Response;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_TOLERANCE_SECONDS: u64 = 300;

/// Why a webhook signature failed to verify. Branch on this rather than the
/// message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebhookErrorReason {
    /// The `Anypost-Signature` header was empty or malformed.
    MalformedHeader,
    /// The header carried no `t=` timestamp.
    NoTimestamp,
    /// The header carried no `v1=` signature.
    NoSignatures,
    /// The timestamp was older than the allowed tolerance.
    TimestampOutOfTolerance,
    /// No signature in the header matched the computed signature.
    NoMatch,
}

/// Raised when a webhook delivery's signature cannot be verified.
#[derive(Clone, Debug)]
pub struct WebhookVerificationError {
    reason: WebhookErrorReason,
    message: String,
}

impl WebhookVerificationError {
    fn new(reason: WebhookErrorReason, message: impl Into<String>) -> Self {
        Self {
            reason,
            message: message.into(),
        }
    }

    /// The machine-readable failure reason.
    pub fn reason(&self) -> WebhookErrorReason {
        self.reason
    }
}

impl fmt::Display for WebhookVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for WebhookVerificationError {}

/// Options for [`verify_with_options`] / [`unwrap_with_options`].
#[derive(Clone, Debug)]
pub struct VerifyOptions {
    /// Reject deliveries older than this many seconds. `0` disables the check.
    pub tolerance_seconds: u64,
    /// Override "now" (Unix seconds) for the freshness check. Mainly for tests.
    pub now: Option<i64>,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self {
            tolerance_seconds: DEFAULT_TOLERANCE_SECONDS,
            now: None,
        }
    }
}

/// Verify a webhook signature with the default options (300s tolerance).
pub fn verify(
    payload: &[u8],
    signature_header: &str,
    secret: &str,
) -> Result<(), WebhookVerificationError> {
    verify_with_options(payload, signature_header, secret, &VerifyOptions::default())
}

/// Verify a webhook signature.
///
/// The header may carry more than one `v1=` component during a secret rotation;
/// a match on any one passes, so deliveries keep verifying across a rotation.
pub fn verify_with_options(
    payload: &[u8],
    signature_header: &str,
    secret: &str,
    options: &VerifyOptions,
) -> Result<(), WebhookVerificationError> {
    let (timestamp, signatures) = parse_header(signature_header)?;

    if options.tolerance_seconds > 0 {
        let now = options.now.unwrap_or_else(current_unix);
        if now - timestamp > options.tolerance_seconds as i64 {
            return Err(WebhookVerificationError::new(
                WebhookErrorReason::TimestampOutOfTolerance,
                format!(
                    "Timestamp {timestamp} is older than the {}s tolerance.",
                    options.tolerance_seconds
                ),
            ));
        }
    }

    let expected = hex_hmac(secret, timestamp, payload);

    // Constant-time over every candidate: accumulate without early exit.
    let mut matched = false;
    for candidate in &signatures {
        if secure_eq(candidate, &expected) {
            matched = true;
        }
    }

    if !matched {
        return Err(WebhookVerificationError::new(
            WebhookErrorReason::NoMatch,
            "No signature in the header matched the computed signature.",
        ));
    }

    Ok(())
}

/// Verify a delivery and return its parsed body as a [`Response`].
pub fn unwrap(
    payload: &[u8],
    signature_header: &str,
    secret: &str,
) -> Result<Response, WebhookVerificationError> {
    unwrap_with_options(payload, signature_header, secret, &VerifyOptions::default())
}

/// Verify a delivery (with options) and return its parsed body as a [`Response`].
pub fn unwrap_with_options(
    payload: &[u8],
    signature_header: &str,
    secret: &str,
    options: &VerifyOptions,
) -> Result<Response, WebhookVerificationError> {
    verify_with_options(payload, signature_header, secret, options)?;
    let value: serde_json::Value = serde_json::from_slice(payload)
        .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
    let value = if value.is_object() {
        value
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };
    Ok(Response::new(value))
}

fn parse_header(header: &str) -> Result<(i64, Vec<String>), WebhookVerificationError> {
    if header.trim().is_empty() {
        return Err(WebhookVerificationError::new(
            WebhookErrorReason::MalformedHeader,
            "The Anypost-Signature header is empty.",
        ));
    }

    let mut timestamp: Option<i64> = None;
    let mut signatures: Vec<String> = Vec::new();

    for part in header.split(',') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "t" => {
                if value.chars().all(|c| c.is_ascii_digit()) && !value.is_empty() {
                    timestamp = value.parse::<i64>().ok();
                }
            }
            "v1" => signatures.push(value.to_string()),
            _ => {}
        }
    }

    let Some(timestamp) = timestamp else {
        return Err(WebhookVerificationError::new(
            WebhookErrorReason::NoTimestamp,
            "The Anypost-Signature header has no timestamp (t=).",
        ));
    };
    if signatures.is_empty() {
        return Err(WebhookVerificationError::new(
            WebhookErrorReason::NoSignatures,
            "The Anypost-Signature header has no v1= signature.",
        ));
    }

    Ok((timestamp, signatures))
}

fn hex_hmac(secret: &str, timestamp: i64, payload: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts a key of any length");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
}

fn secure_eq(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    left.ct_eq(right).into()
}

fn current_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

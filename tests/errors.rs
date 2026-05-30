mod common;

use anypost::{Error, SendEmail};
use common::{client_with, json_headers};
use serde_json::json as j;

fn email() -> SendEmail {
    SendEmail::new("you@x.com", ["a@example.com"]).text("hi")
}

// Retries are disabled so a single queued response maps straight through, even
// for the retryable statuses (429).
async fn send_error(status: u16, body: serde_json::Value, headers: &[(&str, &str)]) -> Error {
    let (client, _) = client_with(vec![json_headers(status, body, headers)], |b| {
        b.max_retries(0)
    });
    client.email.send(&email()).await.unwrap_err()
}

#[tokio::test]
async fn validation_error_exposes_field_problems() {
    let err = send_error(
        422,
        j!({"error": {"type": "validation_error", "message": "Invalid", "errors": {"from": ["required"]}}}),
        &[],
    )
    .await;

    match &err {
        Error::Validation(_) => {}
        other => panic!("expected Validation, got {other:?}"),
    }
    let errors = err.validation_errors().expect("validation errors");
    assert_eq!(errors["from"], vec!["required".to_string()]);
    assert_eq!(err.error_type(), Some("validation_error"));
    assert_eq!(err.status(), Some(422));
}

#[tokio::test]
async fn authentication_error_maps_by_type() {
    let err = send_error(
        401,
        j!({"error": {"type": "authentication_error", "message": "no"}}),
        &[],
    )
    .await;
    assert!(matches!(err, Error::Authentication(_)));
}

#[tokio::test]
async fn not_found_maps_by_type() {
    let err = send_error(
        404,
        j!({"error": {"type": "not_found", "message": "gone"}}),
        &[],
    )
    .await;
    assert!(matches!(err, Error::NotFound(_)));
}

#[tokio::test]
async fn conflict_preserves_specific_type() {
    let err = send_error(
        409,
        j!({"error": {"type": "idempotency_concurrent", "message": "in flight"}}),
        &[],
    )
    .await;
    assert!(matches!(err, Error::Conflict(_)));
    assert_eq!(err.error_type(), Some("idempotency_concurrent"));
}

#[tokio::test]
async fn idempotency_mismatch_has_its_own_variant() {
    let err = send_error(
        422,
        j!({"error": {"type": "idempotency_mismatch", "message": "reused"}}),
        &[],
    )
    .await;
    assert!(matches!(err, Error::IdempotencyMismatch(_)));
}

#[tokio::test]
async fn rate_limit_parses_retry_after() {
    let err = send_error(
        429,
        j!({"error": {"type": "rate_limit_exceeded", "message": "slow down"}}),
        &[("Retry-After", "12")],
    )
    .await;
    assert!(matches!(err, Error::RateLimit(_)));
    assert_eq!(err.retry_after(), Some(12.0));
}

#[tokio::test]
async fn payload_too_large_flat_envelope() {
    // 413 uses the flat envelope: { "error": "payload_too_large" }.
    let err = send_error(413, j!({"error": "payload_too_large"}), &[]).await;
    assert!(matches!(err, Error::PayloadTooLarge(_)));
    assert_eq!(err.error_type(), Some("payload_too_large"));
}

#[tokio::test]
async fn server_error_maps_to_api() {
    let err = send_error(
        500,
        j!({"error": {"type": "internal_error", "message": "boom"}}),
        &[],
    )
    .await;
    assert!(matches!(err, Error::Api(_)));
}

#[tokio::test]
async fn captures_request_id_from_header() {
    let err = send_error(
        404,
        j!({"error": {"type": "not_found", "message": "gone"}}),
        &[("Anypost-Request-Id", "req_abc123")],
    )
    .await;
    assert_eq!(err.request_id(), Some("req_abc123"));
}

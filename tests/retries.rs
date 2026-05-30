mod common;

use anypost::{Error, SendEmail};
use common::{client, client_with, json, network_error};
use serde_json::json as j;

fn email() -> SendEmail {
    SendEmail::new("you@x.com", ["a@example.com"]).text("hi")
}

#[tokio::test]
async fn retries_on_503_then_succeeds() {
    let (client, transport) = client(vec![
        json(
            503,
            j!({"error": {"type": "provisioning_error", "message": "later"}}),
        ),
        json(202, j!({"id": "email_ok"})),
    ]);

    let result = client.email.send(&email()).await.unwrap();
    assert_eq!(result["id"], "email_ok");
    assert_eq!(transport.request_count(), 2);
}

#[tokio::test]
async fn retries_on_network_error() {
    let (client, transport) = client(vec![network_error(), json(202, j!({"id": "email_ok"}))]);
    let result = client.email.send(&email()).await.unwrap();
    assert_eq!(result["id"], "email_ok");
    assert_eq!(transport.request_count(), 2);
}

#[tokio::test]
async fn gives_up_after_max_retries() {
    let (client, transport) = client_with(
        vec![
            json(503, j!({"error": {"type": "x", "message": "1"}})),
            json(503, j!({"error": {"type": "x", "message": "2"}})),
        ],
        |b| b.max_retries(1),
    );

    let err = client.email.send(&email()).await.unwrap_err();
    assert!(matches!(err, Error::Api(_)));
    assert_eq!(transport.request_count(), 2); // initial + 1 retry
}

#[tokio::test]
async fn does_not_retry_client_errors() {
    let (client, transport) = client(vec![json(
        400,
        j!({"error": {"type": "validation_error", "message": "bad"}}),
    )]);
    let _ = client.email.send(&email()).await.unwrap_err();
    assert_eq!(transport.request_count(), 1);
}

#[tokio::test]
async fn reuses_one_idempotency_key_across_retries() {
    let (client, transport) = client(vec![
        json(
            429,
            j!({"error": {"type": "rate_limit_exceeded", "message": "wait"}}),
        ),
        json(202, j!({"id": "email_ok"})),
    ]);
    client.email.send(&email()).await.unwrap();

    let requests = transport.requests();
    assert_eq!(requests.len(), 2);
    let first = requests[0].header("Idempotency-Key").unwrap();
    let second = requests[1].header("Idempotency-Key").unwrap();
    assert_eq!(
        first, second,
        "a retried send must reuse the same idempotency key"
    );
}

#[tokio::test]
async fn network_failure_surfaces_as_connection_error() {
    let (client, _) = client_with(vec![network_error()], |b| b.max_retries(0));
    let err = client.email.send(&email()).await.unwrap_err();
    assert!(matches!(err, Error::Connection { .. }));
    assert_eq!(err.error_type(), Some("connection_error"));
}

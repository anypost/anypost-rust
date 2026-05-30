mod common;

use anypost::transport::Method;
use anypost::{Attachment, BatchEmail, SendEmail};
use common::{client, json};
use serde_json::json as j;

#[tokio::test]
async fn send_serializes_the_request_body() {
    let (client, transport) = client(vec![json(
        202,
        j!({"id": "email_018f", "created_at": "2026-04-30T12:00:00.123Z"}),
    )]);

    let email = client
        .email
        .send(
            &SendEmail::new(
                "Acme <you@yourdomain.com>",
                ["a@example.com", "b@example.com"],
            )
            .cc(["team@example.com"])
            .reply_to("support@yourdomain.com")
            .subject("Receipt")
            .html("<p>Thanks</p>")
            .tag("receipt"),
        )
        .await
        .unwrap();

    assert_eq!(email["id"].as_str(), Some("email_018f"));

    let req = transport.last();
    assert_eq!(req.method, Method::Post);
    assert_eq!(req.path(), "https://api.test/v1/email");

    let body = req.json();
    assert_eq!(body["from"], "Acme <you@yourdomain.com>");
    assert_eq!(body["to"], j!(["a@example.com", "b@example.com"]));
    assert_eq!(body["cc"], j!(["team@example.com"]));
    assert_eq!(body["reply_to"], "support@yourdomain.com");
    assert_eq!(body["subject"], "Receipt");
    assert_eq!(body["html"], "<p>Thanks</p>");
    assert_eq!(body["tags"], j!(["receipt"]));
    // Omitted optional fields are not serialized.
    assert!(body.get("bcc").is_none());
    assert!(body.get("text").is_none());
}

#[tokio::test]
async fn send_sets_an_automatic_idempotency_key() {
    let (client, transport) = client(vec![json(202, j!({"id": "email_1"}))]);
    client
        .email
        .send(&SendEmail::new("you@x.com", ["a@example.com"]).text("hi"))
        .await
        .unwrap();

    let key = transport
        .last()
        .header("Idempotency-Key")
        .map(str::to_string);
    assert!(key.is_some(), "an idempotency key should be generated");
    assert_eq!(key.unwrap().len(), 36, "looks like a uuid");
}

#[tokio::test]
async fn send_honors_an_explicit_idempotency_key() {
    let (client, transport) = client(vec![json(202, j!({"id": "email_1"}))]);
    client
        .email
        .send_with_idempotency_key(
            &SendEmail::new("you@x.com", ["a@example.com"]).text("hi"),
            "order-4823",
        )
        .await
        .unwrap();
    assert_eq!(
        transport.last().header("Idempotency-Key"),
        Some("order-4823")
    );
}

#[tokio::test]
async fn attachments_are_base64_encoded() {
    let (client, transport) = client(vec![json(202, j!({"id": "email_1"}))]);
    client
        .email
        .send(
            &SendEmail::new("you@x.com", ["a@example.com"])
                .subject("Report")
                .text("Attached.")
                .attachment(
                    Attachment::new("hello.txt", b"hello".to_vec()).content_type("text/plain"),
                ),
        )
        .await
        .unwrap();

    let body = transport.last().json();
    let attachment = &body["attachments"][0];
    assert_eq!(attachment["filename"], "hello.txt");
    assert_eq!(attachment["content"], "aGVsbG8="); // base64("hello")
    assert_eq!(attachment["content_type"], "text/plain");
}

#[tokio::test]
async fn batch_returns_mixed_outcomes_without_raising() {
    let response = j!({
        "summary": {"total": 2, "queued": 1, "failed": 1},
        "data": [
            {"status": "queued", "index": 0, "id": "email_1", "created_at": "2026-04-30T12:00:00Z"},
            {"status": "failed", "index": 1, "error": {"type": "validation_error", "message": "bad"}}
        ]
    });
    let (client, transport) = client(vec![json(207, response)]);

    let batch = BatchEmail::new([
        SendEmail::new("you@x.com", ["a@example.com"])
            .subject("A")
            .text(".."),
        SendEmail::new("you@x.com", ["b@example.com"])
            .subject("B")
            .text(".."),
    ]);
    let result = client.email.send_batch(&batch).await.unwrap();

    assert_eq!(result["summary"]["failed"].as_i64(), Some(1));
    assert_eq!(result["data"][0]["status"], "queued");
    assert_eq!(result["data"][1]["error"]["type"], "validation_error");

    let body = transport.last().json();
    assert_eq!(body["emails"].as_array().unwrap().len(), 2);
    assert_eq!(transport.last().path(), "https://api.test/v1/email/batch");
}

#[tokio::test]
async fn batch_entries_omit_from_to_inherit_defaults() {
    let (client, transport) = client(vec![json(202, j!({"summary": {}, "data": []}))]);
    // Entries built with `SendEmail::to` carry no `from`; the batch `defaults`
    // supplies it (matching the dynamic-language SDKs).
    let batch = BatchEmail::new([
        SendEmail::to(["a@example.com"]).subject("Hi A").text(".."),
        SendEmail::to(["b@example.com"]).subject("Hi B").text(".."),
    ])
    .defaults(j!({"from": "you@yourdomain.com"}));
    client.email.send_batch(&batch).await.unwrap();

    let body = transport.last().json();
    assert_eq!(body["defaults"]["from"], "you@yourdomain.com");
    // The entry omits `from` entirely so the default applies.
    assert!(body["emails"][0].get("from").is_none());
    assert_eq!(body["emails"][0]["to"], j!(["a@example.com"]));
}

#[tokio::test]
async fn batch_entry_can_override_from() {
    let (client, transport) = client(vec![json(202, j!({"summary": {}, "data": []}))]);
    let batch = BatchEmail::new([SendEmail::to(["a@example.com"])
        .from("override@example.com")
        .text("..")])
    .defaults(j!({"from": "you@yourdomain.com"}));
    client.email.send_batch(&batch).await.unwrap();

    assert_eq!(
        transport.last().json()["emails"][0]["from"],
        "override@example.com"
    );
}

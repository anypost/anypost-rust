# Anypost Rust SDK

The official Rust crate for the [Anypost](https://anypost.com) email API.

Requires Rust 1.82+. Async by default, built on [reqwest](https://docs.rs/reqwest) and [Tokio](https://tokio.rs). A synchronous client is available behind the `blocking` feature.

## Install

```bash
cargo add anypost
```

Or add it to `Cargo.toml`:

```toml
[dependencies]
anypost = "0.1"
```

## Quickstart

```rust
use anypost::{Client, SendEmail};

#[tokio::main]
async fn main() -> anypost::Result<()> {
    let client = Client::new("ap_your_api_key")?;

    let email = client
        .email
        .send(
            &SendEmail::new("Acme <you@yourdomain.com>", ["someone@example.com"])
                .subject("Hello from Anypost")
                .html("<p>It worked.</p>"),
        )
        .await?;

    println!("{}", email["id"]);
    Ok(())
}
```

`Client::from_env()` reads the key from `ANYPOST_API_KEY` instead. Keep the key server-side; it is a bearer credential.

Requests are built with typed builders: `SendEmail::new(from, to)` followed by chained setters. Responses come back as `anypost::Response`, a thin wrapper over `serde_json::Value` — read fields with indexing (`email["id"].as_str()`), or call `email.deserialize::<T>()` to map into a struct of your own.

## Sending

One of `text`, `html`, or `template_id` is required. All recipients in `to`, `cc`, and `bcc` share one envelope and count against a combined limit of 50.

```rust
client
    .email
    .send(
        &SendEmail::new("Acme <you@yourdomain.com>", ["a@example.com", "b@example.com"])
            .cc(["team@example.com"])
            .reply_to("support@yourdomain.com")
            .subject("Receipt #4823")
            .html("<p>Thanks for your order.</p>")
            .text("Thanks for your order.")
            .tag("receipt"),
    )
    .await?;
```

Attachment `content` is the raw file bytes — pass what `std::fs::read` returns and the SDK base64-encodes it. Do not pre-encode it. The request body is capped at 5 MB.

```rust
use anypost::Attachment;

client
    .email
    .send(
        &SendEmail::new("you@yourdomain.com", ["someone@example.com"])
            .subject("Your report")
            .text("Attached.")
            .attachment(Attachment::new("report.pdf", std::fs::read("report.pdf")?)),
    )
    .await?;
```

Send with a published template and per-recipient variables:

```rust
use serde_json::json;

client
    .email
    .send(
        &SendEmail::new("you@yourdomain.com", ["someone@example.com"])
            .template_id("template_018f2c5e-3a40-7a91-9c25-3a0b1d5e6f78")
            .variables(json!({ "name": "Ada", "plan": "pro" })),
    )
    .await?;
```

## Batch

Send 1 to 100 independent messages in one request. `defaults` fills any field an entry omits. Build entries with `SendEmail::to(...)` to omit `from` (and any other shared field) and let `defaults` supply it; an entry that sets its own value wins.

```rust
use anypost::{BatchEmail, SendEmail};
use serde_json::json;

let batch = BatchEmail::new([
    SendEmail::to(["a@example.com"]).subject("Hi A").text("..."),
    SendEmail::to(["b@example.com"]).subject("Hi B").text("..."),
])
.defaults(json!({ "from": "you@yourdomain.com" }));

let result = client.email.send_batch(&batch).await?;
```

For a standalone send, `SendEmail::new(from, to)` is the constructor — `from` is required there. `SendEmail::to(recipients)` is its sibling for batch entries that inherit a shared sender.

A batch with mixed outcomes returns HTTP `207` and resolves normally. Inspect each entry rather than treating it as an error:

```rust
println!("{}", result["summary"]); // { total, queued, failed }

for entry in result["data"].as_array().unwrap() {
    if entry["status"] == "queued" {
        println!("{} {}", entry["index"], entry["id"]);
    } else {
        println!("{} {} {}", entry["index"], entry["error"]["type"], entry["error"]["message"]);
    }
}
```

## Domains

Manage sending domains under `client.domains`. Add a domain, publish the records it returns, then verify.

```rust
use serde_json::json;

let domain = client.domains.create(json!({ "name": "example.com" })).await?;

for record in domain["dns_records"].as_array().unwrap() {
    println!("{} {} -> {}", record["type"], record["name"], record["value"]);
}
```

`verify` always returns the current domain — a still-`pending` domain is not an error. Read `status` and `verification_failure`, and poll while DNS propagates.

```rust
let checked = client.domains.verify(&domain["id"].as_str().unwrap()).await?;
if checked["status"] != "verified" {
    println!("{}", checked["verification_failure"]["code"]);
}
```

`get`, `update` (tracking config only), and `delete` round out the resource.

## API keys

Manage keys under `client.api_keys`. The plaintext secret comes back only once, on `create`, as `key`:

```rust
use serde_json::json;

let created = client
    .api_keys
    .create(json!({
        "name": "Production server",
        "permissions": "send_only",
        "allowed_domains": ["example.com"]
    }))
    .await?;
println!("{}", created["key"]); // store now; never retrievable again
```

`get` returns metadata only — `key_prefix`, never the secret. Permission and restriction changes take up to 5 minutes to propagate through the gateway cache.

## Templates

Templates use a draft/published model: edits land in a draft, and `publish` promotes it. A template can't be used for sending until it's published.

```rust
use serde_json::json;

let template = client
    .templates
    .create(json!({ "name": "Welcome email", "kind": "html", "html": "<h1>Welcome, {{ name }}</h1>" }))
    .await?;
let id = template["id"].as_str().unwrap();

client
    .templates
    .update_draft(id, json!({ "subject": "Welcome to Acme", "html": "<h1>Welcome, {{ name }}</h1>" }))
    .await?;
client.templates.publish(id).await?;
```

`kind` is `html` or `markdown` and is immutable once set. `get_draft`, `delete_draft`, `duplicate`, `get`, `update` (name only), and `delete` round out the resource. Send with a published template via `template_id` (see [Sending](#sending)).

## Suppressions

A suppression blocks sends to an address, scoped to a `topic`. The wildcard `*` blocks every topic; a specific topic (e.g. `marketing`) leaves transactional traffic untouched. Bounces and complaints write `*` automatically.

```rust
use anypost::SuppressionListParams;
use serde_json::json;

client
    .suppressions
    .create(json!({ "email": "alice@example.com", "topic": "marketing", "note": "Customer requested removal" }))
    .await?;

let row = client.suppressions.get("alice@example.com", "*").await?;
client.suppressions.delete("alice@example.com", "marketing").await?;

let complaints = client
    .suppressions
    .list(SuppressionListParams::new().reason("complaint"))
    .await?;
```

`list_for_email` returns every row for an address across all topics; `delete_for_email` removes them all.

## Webhooks

Manage webhook subscriptions under `client.webhooks`. The `signing_secret` comes back only once, on `create`; later reads return only `signing_secret_prefix`.

```rust
use serde_json::json;

let webhook = client
    .webhooks
    .create(json!({
        "name": "Production events",
        "url": "https://hooks.example.com/anypost",
        "events": ["email.delivered", "email.bounced", "email.complained"]
    }))
    .await?;
println!("{}", webhook["signing_secret"]); // store now; never retrievable again
```

`update` sets the name, URL, events, and `status` together — set `status` to `"disabled"` to pause delivery, `"active"` to resume. `test` sends one synthetic `webhook.test` event and returns the outcome even when the endpoint fails. `rotate_secret` issues a new secret and keeps the previous one valid for a 24-hour grace window; `get`, `list`, and `delete` round out the resource.

### Verifying deliveries

`anypost::webhook::verify` and `unwrap` are free functions — they need the signing secret, not an API key, so call them in your handler without a client. Pass the **raw** request body (the exact bytes, before JSON parsing), the `Anypost-Signature` header, and the secret. `verify` returns `Ok(())` on success; `unwrap` does the same and returns the parsed delivery as a `Response`.

```rust
use anypost::webhook;

match webhook::unwrap(raw_body, signature_header, secret) {
    Ok(delivery) => {
        for event in delivery["events"].as_array().unwrap() {
            // event["type"], event["data"]["email_id"], ...
        }
    }
    Err(e) => {
        // e.reason(): NoMatch | TimestampOutOfTolerance | ...
        return bad_request();
    }
}
```

Reach for `verify` when something else has already parsed the body — keep the raw bytes for the verify step, then use your parsed value once it passes. Deliveries older than five minutes are rejected by default to bound replay; `verify_with_options` widens, narrows, or disables (`0`) that check. During a secret rotation the header carries a `v1=` component per active secret, and a match on any one passes, so deliveries keep verifying while you redeploy.

## Events

`client.events.list` pages the team's event stream, newest-first. The window defaults to the last 24 hours and is clamped to your plan's retention. Events are read-only and not addressable by id — there is no `get`.

```rust
use anypost::EventListParams;

let page = client
    .events
    .list(EventListParams::new().event_type("email.bounced"))
    .await?;

for event in &page.data {
    println!("{} {} {}", event["occurred_at"], event["recipient"], event["bounce_classification"]);
}
```

Filter by `start`, `end`, `event_type`, `recipient`, `email_id`, `message_id`, `domain`, `topic`, `campaign`, `template_id`, and `tags`. All filters are exact-match, except `tags`, which takes a list and matches an event carrying *any* of the given tags. This is also how you backfill the gap after a webhook endpoint was disabled — page the events that occurred during the outage once it's healthy.

## Pagination

List endpoints return a `Page` with `data`, `has_more`, and `next_cursor`. Read one page, pass `next_cursor` back as `after` to fetch the next, or call `list_all` to collect every page.

```rust
use anypost::ListParams;

let page = client.domains.list(ListParams::new().limit(50)).await?;
page.data;        // this page's items
page.has_more;    // whether another page exists
page.next_cursor; // pass to ListParams::after to fetch it yourself

let all = client.domains.list_all(ListParams::new()).await?; // every domain
```

## Errors

A failed request returns an `anypost::Error`. Match on the variant, which corresponds to the stable, machine-readable `error.type`, rather than on the HTTP status.

```rust
use anypost::Error;

match client.email.send(&message).await {
    Ok(email) => { /* email["id"] */ }
    Err(Error::Validation(e)) => { /* e.errors: field -> [problems] */ }
    Err(Error::RateLimit(e)) => { /* e.retry_after: Option<f64> seconds */ }
    Err(e) => eprintln!("{} {:?} {}", e.error_type().unwrap_or("?"), e.status(), e),
}
```

| Variant | `error.type` | Status |
|---|---|---|
| `Validation` | `validation_error` | `400`, `422` |
| `Authentication` | `authentication_error` | `401` |
| `Permission` | `permission_error` | `403` |
| `NotFound` | `not_found` | `404` |
| `Conflict` | `idempotency_concurrent`, `webhook_rotation_in_progress` | `409` |
| `IdempotencyMismatch` | `idempotency_mismatch` | `422` |
| `RateLimit` | `rate_limit_exceeded` | `429` |
| `PayloadTooLarge` | `payload_too_large` | `413` |
| `Api` | `internal_error`, `provisioning_error` | `5xx` |
| `Connection` | `connection_error` | none |

Every API-level error carries `error_type()`, `status()`, `request_id()`, a `message`, and the parsed `raw` body.

## Retries and idempotency

The client retries `429`, `502`, `503`, and network failures up to `max_retries` times (default 2), with exponential backoff and full jitter. It honors `Retry-After`.

Sends are made safe to retry automatically: when retries are enabled and you do not pass an idempotency key, the client generates one and reuses it across attempts, so a retried send cannot deliver twice. Pass your own key to dedupe across process restarts:

```rust
client.email.send_with_idempotency_key(&message, "order-4823").await?;
client.email.send_batch_with_idempotency_key(&batch, "nightly-2026-05-30").await?;
```

## Configuration

```rust
use std::time::Duration;
use anypost::Client;

let client = Client::builder()
    .api_key("ap_your_api_key")
    .base_url("https://api.anypost.com/v1")
    .timeout(Duration::from_secs(30))
    .max_retries(2)
    .default_header("X-My-Header", "value")
    .build()?;
```

| Option | Default | Description |
|---|---|---|
| `base_url` | `https://api.anypost.com/v1` | API base URL. |
| `timeout` | 30s | Per-request timeout. |
| `max_retries` | 2 | Automatic retries for transient failures. |
| `default_header` | none | Extra header sent on every request (repeatable). |

Omit `api_key` to read `ANYPOST_API_KEY` from the environment.

## Blocking client

Enable the `blocking` feature for a synchronous client that owns a Tokio runtime internally:

```toml
[dependencies]
anypost = { version = "0.1", features = ["blocking"] }
```

```rust
use anypost::blocking::Client;
use anypost::SendEmail;

let client = Client::new("ap_your_api_key")?;
let email = client
    .email()
    .send(&SendEmail::new("you@yourdomain.com", ["someone@example.com"]).subject("Hi").text("It worked."))?;
println!("{}", email["id"]);
```

Resources are accessed as methods (`client.email()`, `client.domains()`, …) rather than fields; every method mirrors the async client.

## License

MIT

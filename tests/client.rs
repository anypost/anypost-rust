mod common;

use std::sync::{Mutex, MutexGuard};

use anypost::{Client, Error, SendEmail};
use common::{client, client_with, json};
use serde_json::json as j;

// Serializes the few tests that read/write the process environment.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_guard() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn sample_email() -> SendEmail {
    SendEmail::new("you@yourdomain.com", ["someone@example.com"])
        .subject("Hello")
        .text("Hi")
}

#[tokio::test]
async fn assembles_default_headers() {
    let (client, transport) = client(vec![json(
        202,
        j!({"id": "email_1", "created_at": "2026-04-30T12:00:00Z"}),
    )]);
    client.email.send(&sample_email()).await.unwrap();

    let req = transport.last();
    assert_eq!(req.header("Authorization"), Some("Bearer ap_test_key"));
    assert_eq!(req.header("Accept"), Some("application/json"));
    assert_eq!(req.header("Content-Type"), Some("application/json"));
    assert!(req
        .header("User-Agent")
        .unwrap()
        .starts_with("anypost-rust/"));
    assert_eq!(req.path(), "https://api.test/v1/email");
}

#[tokio::test]
async fn includes_custom_default_headers() {
    let (client, transport) = client_with(vec![json(200, j!({"team": null}))], |b| {
        b.default_header("X-My-Header", "value")
    });
    client.whoami().await.unwrap();
    assert_eq!(
        client_header(&transport, "X-My-Header"),
        Some("value".to_string())
    );
}

fn client_header(transport: &common::MockTransport, name: &str) -> Option<String> {
    transport.last().header(name).map(str::to_string)
}

#[tokio::test]
async fn reads_api_key_from_environment() {
    let _guard = env_guard();
    std::env::set_var("ANYPOST_API_KEY", "ap_env_key");

    let transport = std::sync::Arc::new(MockOnce::new());
    // Build via the public path that hits the environment. Key resolution
    // happens here (sync), so the env lock can be released before the await.
    let client = Client::builder()
        .base_url("https://api.test/v1")
        .transport(transport.clone())
        .build()
        .expect("from env");
    std::env::remove_var("ANYPOST_API_KEY");
    drop(_guard);

    client.whoami().await.unwrap();
    assert_eq!(transport.auth(), Some("Bearer ap_env_key".to_string()));
}

#[tokio::test]
async fn missing_api_key_is_a_config_error() {
    let _guard = env_guard();
    std::env::remove_var("ANYPOST_API_KEY");

    let err = Client::new("").unwrap_err();
    assert!(matches!(err, Error::Config(_)));

    let err = Client::from_env().unwrap_err();
    assert!(matches!(err, Error::Config(_)));
}

// A tiny standalone transport for the env test (records one auth header).
use anypost::transport::{HttpRequest, HttpResponse, Transport, TransportError};
use async_trait::async_trait;

struct MockOnce {
    auth: Mutex<Option<String>>,
}

impl MockOnce {
    fn new() -> Self {
        Self {
            auth: Mutex::new(None),
        }
    }
    fn auth(&self) -> Option<String> {
        self.auth.lock().unwrap().clone()
    }
}

#[async_trait]
impl Transport for MockOnce {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        let auth = request
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
            .map(|(_, v)| v.clone());
        *self.auth.lock().unwrap() = auth;
        Ok(HttpResponse {
            status: 200,
            headers: Vec::new(),
            body: b"{\"team\":null}".to_vec(),
        })
    }
}

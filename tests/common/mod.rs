//! Shared test harness: an injectable mock transport that records requests and
//! replays canned responses, plus a no-op sleeper for deterministic retries.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anypost::transport::{HttpRequest, HttpResponse, Method, Sleeper, Transport, TransportError};
use anypost::{Client, ClientBuilder};
use async_trait::async_trait;
use serde_json::Value;

/// One captured outbound request.
#[derive(Clone, Debug)]
pub struct Recorded {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn json(&self) -> Value {
        let bytes = self.body.as_ref().expect("request had a body");
        serde_json::from_slice(bytes).expect("request body was JSON")
    }

    /// The path portion of the URL, without the query string.
    pub fn path(&self) -> &str {
        self.url.split('?').next().unwrap_or(&self.url)
    }

    /// A query parameter value (no percent-decoding; test values are plain).
    pub fn query(&self, name: &str) -> Option<String> {
        let query = self.url.split_once('?')?.1;
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if key == name {
                    return Some(value.to_string());
                }
            }
        }
        None
    }
}

/// A queued outcome for the mock transport. Construct with the helpers below.
pub enum Canned {
    Response(HttpResponse),
    Error(TransportError),
}

/// A [`Transport`] that records requests and replays queued responses in order.
pub struct MockTransport {
    requests: Mutex<Vec<Recorded>>,
    responses: Mutex<VecDeque<Canned>>,
}

impl MockTransport {
    fn new(responses: Vec<Canned>) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            responses: Mutex::new(responses.into_iter().collect()),
        }
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }

    pub fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }

    pub fn last(&self) -> Recorded {
        self.requests
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("a request was made")
    }

    pub fn first(&self) -> Recorded {
        self.requests
            .lock()
            .unwrap()
            .first()
            .cloned()
            .expect("a request was made")
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        self.requests.lock().unwrap().push(Recorded {
            method: request.method,
            url: request.url,
            headers: request.headers,
            body: request.body,
        });
        match self.responses.lock().unwrap().pop_front() {
            Some(Canned::Response(response)) => Ok(response),
            Some(Canned::Error(error)) => Err(error),
            None => panic!("MockTransport ran out of queued responses"),
        }
    }
}

struct NoopSleeper;

#[async_trait]
impl Sleeper for NoopSleeper {
    async fn sleep(&self, _duration: Duration) {}
    fn jitter(&self) -> f64 {
        1.0
    }
}

/// A JSON response with the given status.
pub fn json(status: u16, body: Value) -> Canned {
    Canned::Response(HttpResponse {
        status,
        headers: Vec::new(),
        body: serde_json::to_vec(&body).unwrap(),
    })
}

/// A JSON response carrying response headers (e.g. Retry-After, request id).
pub fn json_headers(status: u16, body: Value, headers: &[(&str, &str)]) -> Canned {
    Canned::Response(HttpResponse {
        status,
        headers: headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        body: serde_json::to_vec(&body).unwrap(),
    })
}

/// A 204 No Content response.
pub fn no_content() -> Canned {
    Canned::Response(HttpResponse {
        status: 204,
        headers: Vec::new(),
        body: Vec::new(),
    })
}

/// A transport-level (network) failure.
pub fn network_error() -> Canned {
    Canned::Error(TransportError {
        message: "connection reset".to_string(),
        timeout: false,
    })
}

/// Build a client wired to a mock transport (default 2 retries).
pub fn client(responses: Vec<Canned>) -> (Client, Arc<MockTransport>) {
    client_with(responses, |b| b)
}

/// Build a client wired to a mock transport, customizing the builder.
pub fn client_with(
    responses: Vec<Canned>,
    configure: impl FnOnce(ClientBuilder) -> ClientBuilder,
) -> (Client, Arc<MockTransport>) {
    let transport = Arc::new(MockTransport::new(responses));
    let builder = Client::builder()
        .api_key("ap_test_key")
        .base_url("https://api.test/v1")
        .transport(transport.clone())
        .sleeper(Arc::new(NoopSleeper));
    let client = configure(builder).build().expect("client builds");
    (client, transport)
}

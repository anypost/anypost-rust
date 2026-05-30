//! The async client and its builder.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::resources::{
    ApiKeys, Domains, Email, Events, Identity, Suppressions, Templates, Webhooks,
};
use crate::response::Response;
use crate::transport::{RealSleeper, ReqwestTransport, Sleeper, Transport};

const DEFAULT_BASE_URL: &str = "https://api.anypost.com/v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_RETRIES: u32 = 2;
const ENV_KEY: &str = "ANYPOST_API_KEY";

/// Async client for the Anypost email API.
///
/// Resources are public fields: `client.email`, `client.domains`,
/// `client.api_keys`, `client.templates`, `client.suppressions`,
/// `client.webhooks`, and `client.events`.
///
/// ```no_run
/// # async fn run() -> anypost::Result<()> {
/// use anypost::{Client, SendEmail};
///
/// let client = Client::new("ap_your_api_key")?;
/// let email = client
///     .email
///     .send(&SendEmail::new("you@yourdomain.com", ["someone@example.com"])
///         .subject("Hello from Anypost")
///         .html("<p>It worked.</p>"))
///     .await?;
/// println!("{}", email["id"]);
/// # Ok(())
/// # }
/// ```
pub struct Client {
    pub email: Email,
    pub domains: Domains,
    pub api_keys: ApiKeys,
    pub templates: Templates,
    pub suppressions: Suppressions,
    pub webhooks: Webhooks,
    pub events: Events,
    identity: Identity,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Client {
    /// Create a client with the given API key and default configuration.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::builder().api_key(api_key).build()
    }

    /// Create a client, reading the API key from the `ANYPOST_API_KEY`
    /// environment variable.
    pub fn from_env() -> Result<Self> {
        Self::builder().build()
    }

    /// Start a [`ClientBuilder`] for full configuration.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Identify the team and permission level behind the current API key.
    pub async fn whoami(&self) -> Result<Response> {
        self.identity.whoami().await
    }
}

/// Builder for [`Client`].
#[derive(Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    default_headers: Vec<(String, String)>,
    transport: Option<Arc<dyn Transport>>,
    sleeper: Option<Arc<dyn Sleeper>>,
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .finish_non_exhaustive()
    }
}

impl ClientBuilder {
    /// Set the API key. If unset, the key is read from `ANYPOST_API_KEY`.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Override the API base URL (default `https://api.anypost.com/v1`).
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Per-request timeout (default 30s).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Automatic retries for transient failures (default 2).
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Add a header sent on every request.
    pub fn default_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.push((name.into(), value.into()));
        self
    }

    /// Plug in a custom [`Transport`]. Most callers never need this.
    #[doc(hidden)]
    pub fn transport(mut self, transport: Arc<dyn Transport>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Plug in a custom [`Sleeper`]. Mainly for deterministic tests.
    #[doc(hidden)]
    pub fn sleeper(mut self, sleeper: Arc<dyn Sleeper>) -> Self {
        self.sleeper = Some(sleeper);
        self
    }

    /// Build the client, resolving the API key (explicit, else environment).
    pub fn build(self) -> Result<Client> {
        let api_key = self
            .api_key
            .filter(|key| !key.is_empty())
            .or_else(|| std::env::var(ENV_KEY).ok())
            .filter(|key| !key.is_empty())
            .ok_or_else(|| {
                Error::Config(
                    "An Anypost API key is required. Pass it to the builder or set ANYPOST_API_KEY.".to_string(),
                )
            })?;

        let base_url = self
            .base_url
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
            .trim_end_matches('/')
            .to_string();
        let timeout = self.timeout.unwrap_or(DEFAULT_TIMEOUT);
        let max_retries = self.max_retries.unwrap_or(DEFAULT_MAX_RETRIES);

        let transport: Arc<dyn Transport> = match self.transport {
            Some(transport) => transport,
            None => Arc::new(ReqwestTransport::new(timeout).map_err(|e| Error::Config(e.message))?),
        };
        let sleeper: Arc<dyn Sleeper> = self.sleeper.unwrap_or_else(|| Arc::new(RealSleeper));

        let http = Arc::new(HttpClient::new(
            transport,
            sleeper,
            base_url,
            api_key,
            self.default_headers,
            max_retries,
        ));

        Ok(Client {
            email: Email::new(http.clone()),
            domains: Domains::new(http.clone()),
            api_keys: ApiKeys::new(http.clone()),
            templates: Templates::new(http.clone()),
            suppressions: Suppressions::new(http.clone()),
            webhooks: Webhooks::new(http.clone()),
            events: Events::new(http.clone()),
            identity: Identity::new(http),
        })
    }
}

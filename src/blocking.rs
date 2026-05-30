//! A synchronous client, available under the `blocking` feature.
//!
//! Each method mirrors the async client and blocks on a current-thread Tokio
//! runtime owned by the [`Client`]. Most callers want the async
//! [`Client`](crate::Client); reach for this only from synchronous code.
//!
//! ```no_run
//! # #[cfg(feature = "blocking")]
//! # fn run() -> anypost::Result<()> {
//! use anypost::blocking::Client;
//! use anypost::SendEmail;
//!
//! let client = Client::new("ap_your_api_key")?;
//! let email = client.email().send(
//!     &SendEmail::new("you@yourdomain.com", ["someone@example.com"])
//!         .subject("Hello")
//!         .html("<p>It worked.</p>"),
//! )?;
//! println!("{}", email["id"]);
//! # Ok(())
//! # }
//! ```

use serde::Serialize;
use tokio::runtime::Runtime;

use crate::error::{Error, Result};
use crate::response::{Page, Response};
use crate::types::email::{BatchEmail, SendEmail};
use crate::types::params::{EventListParams, ListParams, SuppressionListParams};

/// Synchronous client for the Anypost email API.
pub struct Client {
    inner: crate::Client,
    runtime: Runtime,
}

impl Client {
    /// Create a client with the given API key and default configuration.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::from_async(crate::Client::new(api_key)?)
    }

    /// Create a client, reading the API key from `ANYPOST_API_KEY`.
    pub fn from_env() -> Result<Self> {
        Self::from_async(crate::Client::from_env()?)
    }

    /// Wrap an already-built async [`Client`](crate::Client). Use
    /// [`crate::Client::builder`] for full configuration, then pass the result
    /// here.
    pub fn from_async(inner: crate::Client) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Config(e.to_string()))?;
        Ok(Self { inner, runtime })
    }

    pub fn email(&self) -> Email<'_> {
        Email { client: self }
    }

    pub fn domains(&self) -> Domains<'_> {
        Domains { client: self }
    }

    pub fn api_keys(&self) -> ApiKeys<'_> {
        ApiKeys { client: self }
    }

    pub fn templates(&self) -> Templates<'_> {
        Templates { client: self }
    }

    pub fn suppressions(&self) -> Suppressions<'_> {
        Suppressions { client: self }
    }

    pub fn webhooks(&self) -> Webhooks<'_> {
        Webhooks { client: self }
    }

    pub fn events(&self) -> Events<'_> {
        Events { client: self }
    }

    /// Identify the team and permission level behind the current API key.
    pub fn whoami(&self) -> Result<Response> {
        self.runtime.block_on(self.inner.whoami())
    }
}

/// Send operations (`/email`, `/email/batch`).
pub struct Email<'a> {
    client: &'a Client,
}

impl Email<'_> {
    pub fn send(&self, email: &SendEmail) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.email.send(email))
    }

    pub fn send_with_idempotency_key(&self, email: &SendEmail, key: &str) -> Result<Response> {
        self.client.runtime.block_on(
            self.client
                .inner
                .email
                .send_with_idempotency_key(email, key),
        )
    }

    pub fn send_batch(&self, batch: &BatchEmail) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.email.send_batch(batch))
    }

    pub fn send_batch_with_idempotency_key(
        &self,
        batch: &BatchEmail,
        key: &str,
    ) -> Result<Response> {
        self.client.runtime.block_on(
            self.client
                .inner
                .email
                .send_batch_with_idempotency_key(batch, key),
        )
    }
}

/// Sending-domain operations (`/domains`).
pub struct Domains<'a> {
    client: &'a Client,
}

impl Domains<'_> {
    pub fn list(&self, params: ListParams) -> Result<Page> {
        self.client
            .runtime
            .block_on(self.client.inner.domains.list(params))
    }

    pub fn list_all(&self, params: ListParams) -> Result<Vec<Response>> {
        self.client
            .runtime
            .block_on(self.client.inner.domains.list_all(params))
    }

    pub fn create(&self, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.domains.create(body))
    }

    pub fn get(&self, id: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.domains.get(id))
    }

    pub fn update(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.domains.update(id, body))
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.client
            .runtime
            .block_on(self.client.inner.domains.delete(id))
    }

    pub fn verify(&self, id: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.domains.verify(id))
    }
}

/// API-key operations (`/api-keys`).
pub struct ApiKeys<'a> {
    client: &'a Client,
}

impl ApiKeys<'_> {
    pub fn list(&self, params: ListParams) -> Result<Page> {
        self.client
            .runtime
            .block_on(self.client.inner.api_keys.list(params))
    }

    pub fn list_all(&self, params: ListParams) -> Result<Vec<Response>> {
        self.client
            .runtime
            .block_on(self.client.inner.api_keys.list_all(params))
    }

    pub fn create(&self, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.api_keys.create(body))
    }

    pub fn get(&self, id: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.api_keys.get(id))
    }

    pub fn update(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.api_keys.update(id, body))
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.client
            .runtime
            .block_on(self.client.inner.api_keys.delete(id))
    }
}

/// Template operations, including the draft/publish flow.
pub struct Templates<'a> {
    client: &'a Client,
}

impl Templates<'_> {
    pub fn list(&self, params: ListParams) -> Result<Page> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.list(params))
    }

    pub fn list_all(&self, params: ListParams) -> Result<Vec<Response>> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.list_all(params))
    }

    pub fn create(&self, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.create(body))
    }

    pub fn get(&self, id: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.get(id))
    }

    pub fn update(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.update(id, body))
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.delete(id))
    }

    pub fn duplicate(&self, id: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.duplicate(id))
    }

    pub fn duplicate_with(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.duplicate_with(id, body))
    }

    pub fn get_draft(&self, id: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.get_draft(id))
    }

    pub fn update_draft(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.update_draft(id, body))
    }

    pub fn delete_draft(&self, id: &str) -> Result<()> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.delete_draft(id))
    }

    pub fn publish(&self, id: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.templates.publish(id))
    }
}

/// Suppression-list operations (`/suppressions`).
pub struct Suppressions<'a> {
    client: &'a Client,
}

impl Suppressions<'_> {
    pub fn list(&self, params: SuppressionListParams) -> Result<Page> {
        self.client
            .runtime
            .block_on(self.client.inner.suppressions.list(params))
    }

    pub fn list_all(&self, params: SuppressionListParams) -> Result<Vec<Response>> {
        self.client
            .runtime
            .block_on(self.client.inner.suppressions.list_all(params))
    }

    pub fn create(&self, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.suppressions.create(body))
    }

    pub fn get(&self, email: &str, topic: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.suppressions.get(email, topic))
    }

    pub fn delete(&self, email: &str, topic: &str) -> Result<()> {
        self.client
            .runtime
            .block_on(self.client.inner.suppressions.delete(email, topic))
    }

    pub fn list_for_email(&self, email: &str) -> Result<Vec<Response>> {
        self.client
            .runtime
            .block_on(self.client.inner.suppressions.list_for_email(email))
    }

    pub fn delete_for_email(&self, email: &str) -> Result<()> {
        self.client
            .runtime
            .block_on(self.client.inner.suppressions.delete_for_email(email))
    }
}

/// Webhook operations, including test and rotation.
pub struct Webhooks<'a> {
    client: &'a Client,
}

impl Webhooks<'_> {
    pub fn list(&self, params: ListParams) -> Result<Page> {
        self.client
            .runtime
            .block_on(self.client.inner.webhooks.list(params))
    }

    pub fn list_all(&self, params: ListParams) -> Result<Vec<Response>> {
        self.client
            .runtime
            .block_on(self.client.inner.webhooks.list_all(params))
    }

    pub fn create(&self, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.webhooks.create(body))
    }

    pub fn get(&self, id: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.webhooks.get(id))
    }

    pub fn update(&self, id: &str, body: impl Serialize) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.webhooks.update(id, body))
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.client
            .runtime
            .block_on(self.client.inner.webhooks.delete(id))
    }

    pub fn test(&self, id: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.webhooks.test(id))
    }

    pub fn rotate_secret(&self, id: &str) -> Result<Response> {
        self.client
            .runtime
            .block_on(self.client.inner.webhooks.rotate_secret(id))
    }
}

/// Read access to the event stream (`/events`).
pub struct Events<'a> {
    client: &'a Client,
}

impl Events<'_> {
    pub fn list(&self, params: EventListParams) -> Result<Page> {
        self.client
            .runtime
            .block_on(self.client.inner.events.list(params))
    }

    pub fn list_all(&self, params: EventListParams) -> Result<Vec<Response>> {
        self.client
            .runtime
            .block_on(self.client.inner.events.list_all(params))
    }
}

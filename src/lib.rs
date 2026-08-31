//! Official Rust SDK for the [Anypost](https://anypost.com) email API.
//!
//! Build a [`Client`], then send mail and manage domains, API keys, templates,
//! webhooks, suppressions, and the event stream. The client is async (built on
//! [`reqwest`]); a synchronous [`blocking::Client`] is available under the
//! `blocking` feature.
//!
//! ```no_run
//! # async fn run() -> anypost::Result<()> {
//! use anypost::{Client, SendEmail};
//!
//! let client = Client::new("ap_your_api_key")?;
//!
//! let email = client
//!     .email
//!     .send(
//!         &SendEmail::new("Acme <you@yourdomain.com>", ["someone@example.com"])
//!             .subject("Hello from Anypost")
//!             .html("<p>It worked.</p>"),
//!     )
//!     .await?;
//!
//! println!("queued {}", email["id"]);
//! # Ok(())
//! # }
//! ```
//!
//! Keep the API key server-side. It is a bearer credential.

#![forbid(unsafe_code)]
// Our `Error` enum carries a full `ApiError` (rich response body) by value in
// most variants, which trips clippy's `result_large_err` (>128-byte Err). That
// payload is deliberate — callers read structured error detail directly — and
// errors are off the hot path, so boxing every variant to shave stack bytes
// would churn the public API for no real gain. Accept the larger `Result`.
#![allow(clippy::result_large_err)]

mod client;
mod error;
mod http;
mod resources;
mod response;
mod version;

pub mod transport;
pub mod types;
pub mod webhook;

#[cfg(feature = "blocking")]
pub mod blocking;

pub use client::{Client, ClientBuilder};
pub use error::{ApiError, Error, Result};
pub use response::{Page, Response};
pub use version::VERSION;

pub use resources::{ApiKeys, Domains, Email, Events, Identity, Suppressions, Templates, Webhooks};

pub use types::email::{
    Attachment, AttachmentContent, BatchEmail, ReplyTo, SendEmail, Tracking, Unsubscribe,
    UnsubscribeMode,
};
pub use types::params::{EventListParams, ListParams, SuppressionListParams};

pub use webhook::{
    unwrap, unwrap_with_options, verify, verify_with_options, VerifyOptions, WebhookErrorReason,
    WebhookVerificationError,
};

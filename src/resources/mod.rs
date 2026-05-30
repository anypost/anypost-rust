//! API resources. Each is reachable as a field on [`Client`](crate::Client).

mod api_keys;
mod domains;
mod email;
mod events;
mod identity;
mod suppressions;
mod templates;
mod webhooks;

pub use api_keys::ApiKeys;
pub use domains::Domains;
pub use email::Email;
pub use events::Events;
pub use identity::Identity;
pub use suppressions::Suppressions;
pub use templates::Templates;
pub use webhooks::Webhooks;

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::Serialize;
use serde_json::Value;

use crate::error::{Error, Result};

/// Encode a single path segment, leaving the RFC 3986 unreserved characters
/// (`A-Z a-z 0-9 - _ . ~`) intact and percent-encoding everything else.
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

pub(crate) fn enc(segment: &str) -> String {
    utf8_percent_encode(segment, PATH_SEGMENT).to_string()
}

pub(crate) fn to_value<T: Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value).map_err(|e| Error::Serialization(e.to_string()))
}

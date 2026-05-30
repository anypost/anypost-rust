//! Response wrappers returned by the resources.

use std::fmt;
use std::ops::Deref;

use serde::de::DeserializeOwned;
use serde_json::Value;

/// An immutable view over a decoded JSON response object.
///
/// `Response` derefs to the underlying [`serde_json::Value`], so all of its
/// accessors are available directly — `response["id"].as_str()`, and so on.
///
/// Use [`Response::deserialize`] to map it into a typed struct of your own.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct Response(Value);

impl Response {
    pub(crate) fn new(value: Value) -> Self {
        Response(value)
    }

    /// Borrow the underlying JSON value.
    pub fn as_value(&self) -> &Value {
        &self.0
    }

    /// Consume the response, returning the underlying JSON value.
    pub fn into_value(self) -> Value {
        self.0
    }

    /// Deserialize the response into a typed value.
    pub fn deserialize<T: DeserializeOwned>(&self) -> std::result::Result<T, serde_json::Error> {
        T::deserialize(&self.0)
    }
}

impl Deref for Response {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// One page of a list result.
///
/// To walk every page, pass [`Page::next_cursor`] back as the `after` parameter
/// of the next call, or use the resource's `list_all` helper to collect them
/// all.
#[derive(Clone, Debug)]
pub struct Page {
    /// The items on this page.
    pub data: Vec<Response>,
    /// Whether another page exists.
    pub has_more: bool,
    /// The cursor for the next page, or `None` when `has_more` is false.
    pub next_cursor: Option<String>,
}

impl Page {
    pub(crate) fn from_value(value: Value) -> Self {
        let data = value
            .get("data")
            .and_then(Value::as_array)
            .map(|items| items.iter().cloned().map(Response::new).collect())
            .unwrap_or_default();
        let has_more = value
            .get("has_more")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let next_cursor = value
            .get("next_cursor")
            .and_then(Value::as_str)
            .map(str::to_string);

        Page {
            data,
            has_more,
            next_cursor,
        }
    }
}

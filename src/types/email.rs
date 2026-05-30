//! Typed builders for the send endpoints.
//!
//! Build a [`SendEmail`] with [`SendEmail::new`] and chain setters; pass it to
//! [`Email::send`](crate::Email::send). Attachment `content` is the raw file
//! bytes — the SDK base64-encodes it on the wire. Do not pre-encode it.

use std::collections::BTreeMap;

use base64::Engine;
use serde::{Serialize, Serializer};
use serde_json::Value;

/// One or more `Reply-To` addresses.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum ReplyTo {
    One(String),
    Many(Vec<String>),
}

/// Per-message override of the sending domain's open/click tracking.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Tracking {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opens: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clicks: Option<bool>,
}

impl Tracking {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn opens(mut self, enabled: bool) -> Self {
        self.opens = Some(enabled);
        self
    }

    pub fn clicks(mut self, enabled: bool) -> Self {
        self.clicks = Some(enabled);
        self
    }
}

/// One-click unsubscribe header behavior.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UnsubscribeMode {
    Generate,
    None,
}

/// One-click unsubscribe configuration for a send.
#[derive(Clone, Debug, Serialize)]
pub struct Unsubscribe {
    pub mode: UnsubscribeMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

impl Unsubscribe {
    /// Mint a per-recipient signed token and inject RFC 8058 headers. Requires a
    /// `topic` on the send.
    pub fn generate() -> Self {
        Self {
            mode: UnsubscribeMode::Generate,
            display_name: None,
        }
    }

    /// No header injection — for transactional sends that must not carry
    /// unsubscribe semantics.
    pub fn none() -> Self {
        Self {
            mode: UnsubscribeMode::None,
            display_name: None,
        }
    }

    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }
}

/// An inline attachment. `content` is raw bytes; it is base64-encoded on the wire.
#[derive(Clone, Debug, Serialize)]
pub struct Attachment {
    pub filename: String,
    #[serde(serialize_with = "serialize_base64")]
    pub content: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
}

impl Attachment {
    /// Create an attachment from raw bytes (e.g. `std::fs::read("report.pdf")?`).
    pub fn new(filename: impl Into<String>, content: impl Into<Vec<u8>>) -> Self {
        Self {
            filename: filename.into(),
            content: content.into(),
            content_type: None,
            content_id: None,
        }
    }

    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Set a Content-ID, marking the attachment inline (referenced via `cid:`).
    pub fn content_id(mut self, content_id: impl Into<String>) -> Self {
        self.content_id = Some(content_id.into());
        self
    }
}

fn serialize_base64<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// A single message. Build it with [`SendEmail::new`] and chain setters.
///
/// `from` and `to` are required; one of `text`, `html`, or `template_id` must
/// also be set (enforced by the API).
#[derive(Clone, Debug, Serialize)]
pub struct SendEmail {
    pub from: String,
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cc: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bcc: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<ReplyTo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracking: Option<Tracking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsubscribe: Option<Unsubscribe>,
}

impl SendEmail {
    /// Start a message with the required sender and recipients.
    pub fn new(from: impl Into<String>, to: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            from: from.into(),
            to: to.into_iter().map(Into::into).collect(),
            cc: Vec::new(),
            bcc: Vec::new(),
            reply_to: None,
            subject: None,
            text: None,
            html: None,
            template_id: None,
            headers: None,
            attachments: Vec::new(),
            tags: Vec::new(),
            tracking: None,
            variables: None,
            campaign: None,
            topic: None,
            unsubscribe: None,
        }
    }

    pub fn cc(mut self, addrs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.cc.extend(addrs.into_iter().map(Into::into));
        self
    }

    pub fn bcc(mut self, addrs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.bcc.extend(addrs.into_iter().map(Into::into));
        self
    }

    /// Set a single `Reply-To` address.
    pub fn reply_to(mut self, addr: impl Into<String>) -> Self {
        self.reply_to = Some(ReplyTo::One(addr.into()));
        self
    }

    /// Set multiple `Reply-To` addresses (maximum 10).
    pub fn reply_to_many(mut self, addrs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.reply_to = Some(ReplyTo::Many(addrs.into_iter().map(Into::into).collect()));
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    pub fn template_id(mut self, template_id: impl Into<String>) -> Self {
        self.template_id = Some(template_id.into());
        self
    }

    /// Add a single custom header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .get_or_insert_with(BTreeMap::new)
            .insert(name.into(), value.into());
        self
    }

    /// Add several custom headers.
    pub fn headers(
        mut self,
        headers: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        let map = self.headers.get_or_insert_with(BTreeMap::new);
        for (name, value) in headers {
            map.insert(name.into(), value.into());
        }
        self
    }

    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn attachments(mut self, attachments: impl IntoIterator<Item = Attachment>) -> Self {
        self.attachments.extend(attachments);
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    pub fn tracking(mut self, tracking: Tracking) -> Self {
        self.tracking = Some(tracking);
        self
    }

    /// Set the per-send substitution map. Accepts anything `Serialize`, e.g. a
    /// `serde_json::json!({ ... })` or your own struct.
    pub fn variables<T: Serialize>(mut self, variables: T) -> Self {
        self.variables = serde_json::to_value(variables).ok();
        self
    }

    pub fn campaign(mut self, campaign: impl Into<String>) -> Self {
        self.campaign = Some(campaign.into());
        self
    }

    pub fn topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = Some(topic.into());
        self
    }

    pub fn unsubscribe(mut self, unsubscribe: Unsubscribe) -> Self {
        self.unsubscribe = Some(unsubscribe);
        self
    }
}

/// A batch of 1–100 messages. `defaults` fills any field an entry omits (`to`
/// is always per-entry).
#[derive(Clone, Debug, Serialize)]
pub struct BatchEmail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<Value>,
    pub emails: Vec<SendEmail>,
}

impl BatchEmail {
    pub fn new(emails: impl IntoIterator<Item = SendEmail>) -> Self {
        Self {
            defaults: None,
            emails: emails.into_iter().collect(),
        }
    }

    /// Set batch-wide defaults. Accepts anything `Serialize`, e.g.
    /// `serde_json::json!({ "from": "you@yourdomain.com" })`.
    pub fn defaults<T: Serialize>(mut self, defaults: T) -> Self {
        self.defaults = serde_json::to_value(defaults).ok();
        self
    }

    pub fn push(mut self, email: SendEmail) -> Self {
        self.emails.push(email);
        self
    }
}

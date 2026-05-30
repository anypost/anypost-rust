//! Parameter builders for list endpoints.

/// Pagination parameters shared by most list endpoints.
#[derive(Clone, Debug, Default)]
pub struct ListParams {
    pub limit: Option<u32>,
    pub after: Option<String>,
}

impl ListParams {
    pub fn new() -> Self {
        Self::default()
    }

    /// Items per page, 1 to 100 (default 20).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// A cursor from a previous page's `next_cursor`.
    pub fn after(mut self, cursor: impl Into<String>) -> Self {
        self.after = Some(cursor.into());
        self
    }
}

/// Filters for `client.suppressions.list`.
#[derive(Clone, Debug, Default)]
pub struct SuppressionListParams {
    pub limit: Option<u32>,
    pub after: Option<String>,
    pub email_contains: Option<String>,
    pub topic: Option<String>,
    pub reason: Option<String>,
    pub origin: Option<String>,
}

impl SuppressionListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn after(mut self, cursor: impl Into<String>) -> Self {
        self.after = Some(cursor.into());
        self
    }

    pub fn email_contains(mut self, value: impl Into<String>) -> Self {
        self.email_contains = Some(value.into());
        self
    }

    pub fn topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = Some(topic.into());
        self
    }

    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = Some(origin.into());
        self
    }
}

/// Filters for `client.events.list`. The window defaults to the last 24 hours
/// and is clamped to the plan's retention.
#[derive(Clone, Debug, Default)]
pub struct EventListParams {
    pub limit: Option<u32>,
    pub after: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub event_type: Option<String>,
    pub recipient: Option<String>,
    pub email_id: Option<String>,
    pub message_id: Option<String>,
    pub domain: Option<String>,
    pub topic: Option<String>,
    pub campaign: Option<String>,
    pub template_id: Option<String>,
    /// Matched with `hasAny`: an event carrying any of these tags matches.
    pub tags: Vec<String>,
}

impl EventListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn after(mut self, cursor: impl Into<String>) -> Self {
        self.after = Some(cursor.into());
        self
    }

    pub fn start(mut self, start: impl Into<String>) -> Self {
        self.start = Some(start.into());
        self
    }

    pub fn end(mut self, end: impl Into<String>) -> Self {
        self.end = Some(end.into());
        self
    }

    pub fn event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    pub fn recipient(mut self, recipient: impl Into<String>) -> Self {
        self.recipient = Some(recipient.into());
        self
    }

    pub fn email_id(mut self, email_id: impl Into<String>) -> Self {
        self.email_id = Some(email_id.into());
        self
    }

    pub fn message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    pub fn topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = Some(topic.into());
        self
    }

    pub fn campaign(mut self, campaign: impl Into<String>) -> Self {
        self.campaign = Some(campaign.into());
        self
    }

    pub fn template_id(mut self, template_id: impl Into<String>) -> Self {
        self.template_id = Some(template_id.into());
        self
    }

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }
}

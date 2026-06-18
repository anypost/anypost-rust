mod common;

use anypost::EventListParams;
use common::{client, json};
use serde_json::json as j;

#[tokio::test]
async fn list_threads_event_type_and_tags() {
    let (client, transport) = client(vec![json(
        200,
        j!({"data": [], "has_more": false, "next_cursor": null}),
    )]);

    client
        .events
        .list(
            EventListParams::new()
                .event_type("email.bounced")
                .tags(["welcome", "onboarding"]),
        )
        .await
        .unwrap();

    assert_eq!(
        transport.last().query("event_type").as_deref(),
        Some("email.bounced")
    );
    // Sent comma-separated (URL-encoded `%2C`); the API matches with hasAny.
    assert_eq!(
        transport.last().query("tags").as_deref(),
        Some("welcome%2Conboarding")
    );
}

#[tokio::test]
async fn exposes_bot_on_proxied_open() {
    let (client, _) = client(vec![json(
        200,
        j!({
            "data": [
                {"id": "evt_bot", "type": "email.opened", "tracking": {"bot": {"source": "google", "kind": "proxy"}}},
                {"id": "evt_human", "type": "email.opened", "tracking": null}
            ],
            "has_more": false,
            "next_cursor": null
        }),
    )]);

    let page = client
        .events
        .list(EventListParams::new().event_type("email.opened"))
        .await
        .unwrap();

    // Events are dynamic Responses (serde_json::Value); the nested bot object,
    // mirroring the webhook payload's data.tracking, is reachable by index.
    assert_eq!(page.data[0]["tracking"]["bot"]["source"], "google");
    assert_eq!(page.data[0]["tracking"]["bot"]["kind"], "proxy");
    // A human open carries no bot classification.
    assert!(page.data[1]["tracking"].is_null());
}

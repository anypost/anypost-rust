mod common;

use anypost::ListParams;
use common::{client, json};
use serde_json::json as j;

#[tokio::test]
async fn list_returns_one_page() {
    let (client, transport) = client(vec![json(
        200,
        j!({
            "data": [{"id": "domain_1", "name": "a.com"}, {"id": "domain_2", "name": "b.com"}],
            "has_more": true,
            "next_cursor": "cursor_2"
        }),
    )]);

    let page = client
        .domains
        .list(ListParams::new().limit(50))
        .await
        .unwrap();
    assert_eq!(page.data.len(), 2);
    assert_eq!(page.data[0]["name"], "a.com");
    assert!(page.has_more);
    assert_eq!(page.next_cursor.as_deref(), Some("cursor_2"));
    assert_eq!(transport.last().query("limit").as_deref(), Some("50"));
}

#[tokio::test]
async fn list_all_walks_every_page() {
    let (client, transport) = client(vec![
        json(
            200,
            j!({"data": [{"id": "d1"}], "has_more": true, "next_cursor": "c1"}),
        ),
        json(
            200,
            j!({"data": [{"id": "d2"}, {"id": "d3"}], "has_more": false, "next_cursor": null}),
        ),
    ]);

    let all = client.domains.list_all(ListParams::new()).await.unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[2]["id"], "d3");

    let requests = transport.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].query("after").is_none());
    assert_eq!(requests[1].query("after").as_deref(), Some("c1"));
}

#[tokio::test]
async fn list_for_email_collects_rows() {
    let (client, _) = client(vec![json(
        200,
        j!({"data": [{"email": "a@x.com", "topic": "*"}, {"email": "a@x.com", "topic": "marketing"}]}),
    )]);
    let rows = client.suppressions.list_for_email("a@x.com").await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["topic"], "marketing");
}

#[tokio::test]
async fn delete_returns_unit_on_204() {
    let (client, transport) = client(vec![common::no_content()]);
    client.domains.delete("domain_1").await.unwrap();
    assert_eq!(
        transport.last().path(),
        "https://api.test/v1/domains/domain_1"
    );
}

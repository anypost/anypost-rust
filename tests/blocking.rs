#![cfg(feature = "blocking")]

mod common;

use anypost::blocking::Client as BlockingClient;
use anypost::{ListParams, SendEmail};
use common::{client, json, no_content};
use serde_json::json as j;

// These are plain `#[test]`s (no ambient async runtime): the blocking client
// owns its own runtime and would panic if nested inside one.

#[test]
fn blocking_send_works() {
    let (async_client, transport) = client(vec![json(202, j!({"id": "email_block"}))]);
    let blocking = BlockingClient::from_async(async_client).unwrap();

    let email = blocking
        .email()
        .send(
            &SendEmail::new("you@x.com", ["a@example.com"])
                .subject("Hi")
                .text("hello"),
        )
        .unwrap();

    assert_eq!(email["id"], "email_block");
    assert_eq!(transport.last().path(), "https://api.test/v1/email");
    assert!(transport.last().header("Idempotency-Key").is_some());
}

#[test]
fn blocking_list_and_delete() {
    let (async_client, transport) = client(vec![
        json(
            200,
            j!({"data": [{"id": "domain_1"}], "has_more": false, "next_cursor": null}),
        ),
        no_content(),
    ]);
    let blocking = BlockingClient::from_async(async_client).unwrap();

    let page = blocking.domains().list(ListParams::new()).unwrap();
    assert_eq!(page.data.len(), 1);

    blocking.domains().delete("domain_1").unwrap();
    assert_eq!(transport.request_count(), 2);
}

#[test]
fn blocking_whoami() {
    let (async_client, _) = client(vec![json(
        200,
        j!({"team": {"id": "team_1", "name": "Acme"}}),
    )]);
    let blocking = BlockingClient::from_async(async_client).unwrap();
    let me = blocking.whoami().unwrap();
    assert_eq!(me["team"]["name"], "Acme");
}

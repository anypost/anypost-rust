mod common;

use anypost::transport::Method;
use common::{client, json};
use serde_json::json as j;

#[tokio::test]
async fn create_batch_wraps_items_and_parses_the_result() {
    let (client, transport) = client(vec![json(
        200,
        j!({
            "imported": 1,
            "already_suppressed": 0,
            "invalid": [{"index": 1, "email": "not-an-address"}]
        }),
    )]);

    let result = client
        .suppressions
        .create_batch(j!([
            {"email": "alice@example.com", "topic": "marketing", "reason": "hard bounce"},
            {"email": "not-an-address"},
        ]))
        .await
        .unwrap();

    assert_eq!(result["imported"], 1);
    assert_eq!(result["already_suppressed"], 0);
    assert_eq!(result["invalid"][0]["index"], 1);
    assert_eq!(result["invalid"][0]["email"], "not-an-address");

    let req = transport.last();
    assert_eq!(req.method, Method::Post);
    assert_eq!(req.path(), "https://api.test/v1/suppressions/batch");
    assert_eq!(
        req.json(),
        j!({"suppressions": [
            {"email": "alice@example.com", "topic": "marketing", "reason": "hard bounce"},
            {"email": "not-an-address"},
        ]})
    );
}

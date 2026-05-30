use anypost::{
    unwrap_with_options, verify, verify_with_options, VerifyOptions, WebhookErrorReason,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const SECRET: &str = "whsec_test_secret";

fn sign(secret: &str, timestamp: i64, payload: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
}

fn at(timestamp: i64) -> VerifyOptions {
    VerifyOptions {
        tolerance_seconds: 300,
        now: Some(timestamp),
    }
}

#[test]
fn verifies_a_valid_signature() {
    let payload = br#"{"events":[]}"#;
    let header = format!("t=1000,v1={}", sign(SECRET, 1000, payload));
    assert!(verify_with_options(payload, &header, SECRET, &at(1000)).is_ok());
}

#[test]
fn accepts_any_matching_signature_during_rotation() {
    let payload = br#"{"events":[]}"#;
    let good = sign(SECRET, 1000, payload);
    let header = format!("t=1000,v1=deadbeefdeadbeef,v1={good}");
    assert!(verify_with_options(payload, &header, SECRET, &at(1000)).is_ok());
}

#[test]
fn rejects_a_tampered_signature() {
    let payload = br#"{"events":[]}"#;
    let header = format!("t=1000,v1={}", sign(SECRET, 1000, b"different"));
    let err = verify_with_options(payload, &header, SECRET, &at(1000)).unwrap_err();
    assert_eq!(err.reason(), WebhookErrorReason::NoMatch);
}

#[test]
fn rejects_an_out_of_tolerance_timestamp() {
    let payload = br#"{"events":[]}"#;
    let header = format!("t=1000,v1={}", sign(SECRET, 1000, payload));
    // now is 301s after the signed timestamp; tolerance is 300.
    let err = verify_with_options(payload, &header, SECRET, &at(1301)).unwrap_err();
    assert_eq!(err.reason(), WebhookErrorReason::TimestampOutOfTolerance);
}

#[test]
fn tolerance_zero_disables_the_freshness_check() {
    let payload = br#"{"events":[]}"#;
    let header = format!("t=1000,v1={}", sign(SECRET, 1000, payload));
    let options = VerifyOptions {
        tolerance_seconds: 0,
        now: Some(9_999_999),
    };
    assert!(verify_with_options(payload, &header, SECRET, &options).is_ok());
}

#[test]
fn rejects_a_malformed_header() {
    let err = verify(b"{}", "", SECRET).unwrap_err();
    assert_eq!(err.reason(), WebhookErrorReason::MalformedHeader);
}

#[test]
fn requires_a_timestamp() {
    let err = verify_with_options(b"{}", "v1=abc", SECRET, &at(1000)).unwrap_err();
    assert_eq!(err.reason(), WebhookErrorReason::NoTimestamp);
}

#[test]
fn requires_a_signature() {
    let err = verify_with_options(b"{}", "t=1000", SECRET, &at(1000)).unwrap_err();
    assert_eq!(err.reason(), WebhookErrorReason::NoSignatures);
}

#[test]
fn unwrap_returns_the_parsed_body() {
    let payload = br#"{"events":[{"type":"email.delivered","data":{"email_id":"email_1"}}]}"#;
    let header = format!("t=1000,v1={}", sign(SECRET, 1000, payload));
    let delivery = unwrap_with_options(payload, &header, SECRET, &at(1000)).unwrap();
    assert_eq!(delivery["events"][0]["type"], "email.delivered");
    assert_eq!(delivery["events"][0]["data"]["email_id"], "email_1");
}

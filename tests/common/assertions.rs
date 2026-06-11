use reqwest::StatusCode;
use serde_json::Value;

use super::client::TestResponse;

/// Assert the response status, surfacing the body on mismatch.
#[track_caller]
pub fn assert_status(res: &TestResponse, expected: StatusCode) {
    assert_eq!(res.status, expected, "unexpected status (body: {})", res.body);
}

/// Assert an error response: the status matches and the body honors the
/// `{ "status", "reason" }` error contract every handler returns.
#[track_caller]
pub fn assert_error(res: &TestResponse, expected: StatusCode) {
    assert_status(res, expected);
    assert!(
        res.body.get("reason").and_then(Value::as_str).is_some(),
        "error response missing string `reason` (body: {})",
        res.body
    );
}

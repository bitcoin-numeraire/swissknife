use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{Client, Method, StatusCode};
use serde_json::Value;

/// Credentials to attach to a request.
#[derive(Clone, Copy, Default)]
pub enum Auth<'a> {
    #[default]
    None,
    /// JWT, sent as `Authorization: Bearer <token>`.
    Bearer(&'a str),
    /// API key, sent base64-encoded in the `api-key` header (as the middleware expects).
    ApiKey(&'a str),
}

/// A decoded HTTP response: status plus the parsed JSON body (`Null` when the
/// body is empty). Pre-parsing keeps assertions and field access terse.
pub struct TestResponse {
    pub status: StatusCode,
    pub body: Value,
}

/// Thin HTTP client bound to one running SwissKnife instance. Cheap to create,
/// so each test makes its own (avoids sharing a `reqwest::Client` across the
/// per-test Tokio runtimes).
#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    http: Client,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http: Client::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Low-level GET that surfaces transport errors (used by readiness polling).
    pub async fn try_get(&self, path: &str) -> reqwest::Result<TestResponse> {
        Ok(decode(self.http.get(self.url(path)).send().await?).await)
    }

    pub async fn request(&self, method: Method, path: &str, auth: Auth<'_>, body: Option<Value>) -> TestResponse {
        let mut req = self.http.request(method, self.url(path));
        req = match auth {
            Auth::None => req,
            Auth::Bearer(token) => req.bearer_auth(token),
            Auth::ApiKey(key) => req.header("api-key", STANDARD.encode(key)),
        };
        if let Some(body) = body {
            req = req.json(&body);
        }
        decode(req.send().await.expect("HTTP request should complete")).await
    }

    pub async fn get(&self, path: &str, auth: Auth<'_>) -> TestResponse {
        self.request(Method::GET, path, auth, None).await
    }

    pub async fn post(&self, path: &str, auth: Auth<'_>, body: Value) -> TestResponse {
        self.request(Method::POST, path, auth, Some(body)).await
    }

    pub async fn put(&self, path: &str, auth: Auth<'_>, body: Value) -> TestResponse {
        self.request(Method::PUT, path, auth, Some(body)).await
    }

    pub async fn delete(&self, path: &str, auth: Auth<'_>) -> TestResponse {
        self.request(Method::DELETE, path, auth, None).await
    }
}

async fn decode(resp: reqwest::Response) -> TestResponse {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let body = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap_or(Value::String(text))
    };
    TestResponse { status, body }
}

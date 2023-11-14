use axum::{routing::get, Router};

pub struct RGBController;

impl RGBController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn routes(self) -> Router {
        Router::new().route("/", get(Self::index))
    }

    async fn index() -> &'static str {
        "Hello, World!"
    }
}

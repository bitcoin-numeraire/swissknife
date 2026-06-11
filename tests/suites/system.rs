//! `/v1/system/*` — health, readiness, version, setup. Public, no auth.

use reqwest::StatusCode;

use crate::common::{app, assert_status, Auth};

mod ready {
    use super::*;

    #[tokio::test]
    async fn returns_204() {
        let app = app().await;
        let res = app.api().get("/v1/system/ready", Auth::None).await;
        assert_status(&res, StatusCode::NO_CONTENT);
    }
}

mod health {
    use super::*;

    #[tokio::test]
    async fn reports_operational_dependencies() {
        let app = app().await;
        let res = app.api().get("/v1/system/health", Auth::None).await;
        assert_status(&res, StatusCode::OK);
        assert_eq!(res.body["is_healthy"], true, "{}", res.body);
        assert_eq!(res.body["database"], "Operational", "{}", res.body);
        assert_eq!(res.body["ln_provider"], "Operational", "{}", res.body);
    }
}

mod version {
    use super::*;

    #[tokio::test]
    async fn exposes_the_package_version() {
        let app = app().await;
        let res = app.api().get("/v1/system/version", Auth::None).await;
        assert_status(&res, StatusCode::OK);
        assert!(res.body["version"].as_str().is_some(), "{}", res.body);
    }
}

mod setup {
    use super::*;

    #[tokio::test]
    async fn reports_setup_flags() {
        let app = app().await;
        let res = app.api().get("/v1/system/setup", Auth::None).await;
        assert_status(&res, StatusCode::OK);
        assert!(res.body["welcome_complete"].is_boolean(), "{}", res.body);
        assert!(res.body["sign_up_complete"].is_boolean(), "{}", res.body);
    }
}

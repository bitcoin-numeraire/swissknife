//! `/v1/system/*` — health, readiness, version, setup. Public, no auth.

use reqwest::StatusCode;

use swissknife_types::{HealthCheck, HealthStatus, SetupInfo, VersionInfo};

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
        let health = res.parse::<HealthCheck>();
        assert!(health.is_healthy, "{health:?}");
        assert_eq!(health.database, HealthStatus::Operational);
        assert_eq!(health.ln_provider, HealthStatus::Operational);
    }
}

mod version {
    use super::*;

    #[tokio::test]
    async fn exposes_the_package_version() {
        let app = app().await;
        let res = app.api().get("/v1/system/version", Auth::None).await;
        assert_status(&res, StatusCode::OK);
        assert!(!res.parse::<VersionInfo>().version.is_empty(), "{}", res.body);
    }
}

mod setup {
    use super::*;

    #[tokio::test]
    async fn reports_setup_flags() {
        let app = app().await;
        let res = app.api().get("/v1/system/setup", Auth::None).await;
        assert_status(&res, StatusCode::OK);
        let info = res.parse::<SetupInfo>();
        assert!(info.sign_up_complete, "admin is created at startup: {info:?}");
    }
}

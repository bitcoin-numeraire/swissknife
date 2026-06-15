use std::fs;
use std::path::Path;

/// An isolated test database. SwissKnife runs migrations on connect, so all the
/// harness has to do is hand it a clean, unique URL.
pub struct TestDatabase {
    url: String,
}

impl TestDatabase {
    /// Provision an isolated database for one spawned instance. `label`
    /// disambiguates the backing file/db name so several instances in the same
    /// test process (e.g. the JWT app and the OAuth2 app) never collide.
    pub async fn provision(kind: &str, root: &Path, label: &str) -> Self {
        match kind {
            "sqlite" => Self::sqlite(root, label),
            "postgres" => Self::postgres(label).await,
            other => panic!("unknown SWISSKNIFE_ITEST_DATABASE '{other}' (expected `sqlite` or `postgres`)"),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// A fresh sqlite file per test-binary process (and `label`), under
    /// `target/itest`.
    fn sqlite(root: &Path, label: &str) -> Self {
        let dir = root.join("target/itest");
        fs::create_dir_all(&dir).expect("create sqlite dir");
        let path = dir.join(format!("itest-{}-{label}.db", std::process::id()));
        for suffix in ["", "-wal", "-shm"] {
            let _ = fs::remove_file(format!("{}{suffix}", path.display()));
        }
        Self {
            url: format!("sqlite://{}?mode=rwc", path.display()),
        }
    }

    /// A fresh database on the dockerized Postgres, dropped/recreated so
    /// migrations run from clean. Connection details are overridable via env
    /// for non-default stacks.
    async fn postgres(label: &str) -> Self {
        use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};

        let admin_url = std::env::var("SWISSKNIFE_ITEST_POSTGRES_ADMIN_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/postgres".to_string());
        let base_url = std::env::var("SWISSKNIFE_ITEST_POSTGRES_BASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432".to_string());
        let name = format!("itest_{}_{}", std::process::id(), label.replace('-', "_"));

        let admin = Database::connect(&admin_url)
            .await
            .expect("connect to postgres admin db");
        for stmt in [
            format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)"),
            format!("CREATE DATABASE \"{name}\""),
        ] {
            admin
                .execute(Statement::from_string(DatabaseBackend::Postgres, stmt))
                .await
                .expect("provision postgres test database");
        }

        Self {
            url: format!("{}/{name}", base_url.trim_end_matches('/')),
        }
    }
}

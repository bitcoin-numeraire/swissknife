use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    // Help and argument errors must work without deployment configuration.
    let directory = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_swissknife"))
        .current_dir(directory.path())
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_lists_recovery_without_loading_config() {
    let result = run(&["--help"]);
    assert!(result.status.success());
    let help = String::from_utf8(result.stdout).unwrap();
    assert!(help.contains("recover-local-login"));
    assert!(help.contains("check-ledger-migrations"));
    assert!(help.contains("--version"));
}

#[test]
fn recovery_help_describes_the_account_argument() {
    let result = run(&["recover-local-login", "--help"]);
    assert!(result.status.success());
    let help = String::from_utf8(result.stdout).unwrap();
    assert!(help.contains("<ACCOUNT_ID>"));
    assert!(help.contains("without connecting to a Lightning provider"));
}

#[test]
fn version_works_without_loading_config() {
    let result = run(&["--version"]);
    assert!(result.status.success());
    assert!(String::from_utf8(result.stdout)
        .unwrap()
        .contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn invalid_arguments_exit_with_a_usage_error() {
    for args in [
        vec!["unknown-command"],
        vec!["recover-local-login"],
        vec!["recover-local-login", "not-a-uuid"],
        vec!["recover-local-login", "00000000-0000-0000-0000-000000000001", "extra"],
    ] {
        let result = run(&args);
        assert_eq!(result.status.code(), Some(2), "{args:?}");
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains("error:"), "{error}");
        assert!(!error.contains("Failed to load config"));
    }
}

fn check_migrations(url: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_swissknife"))
        .env("RUN_MODE", "production")
        .env("SWISSKNIFE_DATABASE__URL", url)
        .arg("check-ledger-migrations")
        .output()
        .unwrap()
}

#[test]
fn checking_a_missing_sqlite_file_does_not_create_it() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing.sqlite");
    let result = check_migrations(&format!("sqlite://{}?mode=rwc", path.display()));
    assert!(!result.status.success());
    assert!(!path.exists());
}

#[tokio::test]
async fn checks_release_migrations_without_changing_the_restored_ledger() {
    use migration::{Migrator, MigratorTrait, SchemaManager};
    use sea_orm::{ConnectionTrait, Database};

    let directory = tempfile::tempdir().unwrap();
    let kind = std::env::var("SWISSKNIFE_ITEST_DATABASE").unwrap_or_else(|_| "sqlite".into());
    let target = crate::common::db::TestDatabase::provision(&kind, directory.path(), "restore_check").await;
    let db = Database::connect(target.url()).await.unwrap();

    let result = check_migrations(target.url());
    assert!(!result.status.success());
    assert!(!SchemaManager::new(&db).has_table("seaql_migrations").await.unwrap());

    Migrator::up(&db, Some(1)).await.unwrap();
    let result = check_migrations(target.url());
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Unapplied migrations"));
    assert_eq!(Migrator::get_applied_migrations_read_only(&db).await.unwrap().len(), 1);

    Migrator::up(&db, None).await.unwrap();
    let result = check_migrations(target.url());
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert!(String::from_utf8_lossy(&result.stdout).contains("matches this SwissKnife binary"));

    db.execute_unprepared("INSERT INTO seaql_migrations (version, applied_at) VALUES ('newer_release', 0)")
        .await
        .unwrap();
    let result = check_migrations(target.url());
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("newer_release"));
}

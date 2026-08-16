use migration::{Migrator, MigratorTrait};
use sea_orm_migration::sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};

async fn sqlite() -> DatabaseConnection {
    Database::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite")
}

async fn count(conn: &DatabaseConnection, sql: &str) -> i64 {
    conn.query_one_raw(Statement::from_string(DatabaseBackend::Sqlite, sql.to_owned()))
        .await
        .expect("query count")
        .expect("count row")
        .try_get::<i64>("", "count")
        .expect("count value")
}

#[test]
fn renamed_modules_preserve_deployed_migration_names() {
    let names = Migrator::migrations()
        .into_iter()
        .take(6)
        .map(|migration| migration.name().to_owned())
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        [
            "m20240420_1_wallet_table",
            "m20240420_2_ln_address_table",
            "m20240420_3_invoice_table",
            "m20240420_4_payment_table",
            "m20241005_5_ln_address_nostr",
            "m20241009_6_api_key_table",
        ]
    );
}

#[tokio::test]
async fn fresh_sqlite_schema_preserves_migration_contracts() {
    let conn = sqlite().await;

    Migrator::up(&conn, None).await.expect("run migrations");

    assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM asset").await, 6);
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) AS count FROM asset WHERE protocol = 'bitcoin' AND asset_ref = 'native'",
        )
        .await,
        6
    );
    assert_eq!(
        count(
            &conn,
            r#"
            SELECT COUNT(*) AS count
            FROM pragma_foreign_key_list('ln_address')
            WHERE "table" = 'wallet' AND "on_delete" = 'CASCADE'
            "#,
        )
        .await,
        2
    );
    assert_eq!(
        count(
            &conn,
            r#"
            SELECT COUNT(*) AS count
            FROM pragma_foreign_key_list('client_event')
            WHERE "table" = 'wallet'
              AND "from" = 'wallet_id'
              AND "on_delete" = 'CASCADE'
            "#,
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &conn,
            r#"
            SELECT COUNT(*) AS count
            FROM sqlite_master
            WHERE type = 'index'
              AND name IN (
                'idx_client_event_created_at',
                'idx_client_event_type_resource',
                'idx_client_event_wallet_id'
              )
            "#,
        )
        .await,
        3
    );
    assert!(
        count(
            &conn,
            r#"
            SELECT COUNT(*) AS count
            FROM pragma_index_list('wallet')
            WHERE "unique" = 1
            "#,
        )
        .await
            >= 2
    );
    assert_eq!(
        count(
            &conn,
            r#"
            SELECT COUNT(*) AS count
            FROM pragma_table_info('wallet')
            WHERE name IN ('account_id', 'asset_id', 'available_amount', 'reserved_amount')
              AND "notnull" = 1
            "#,
        )
        .await,
        4
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = 'wallet_balance'",
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) AS count FROM pragma_table_info('payment') WHERE name = 'currency'",
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) AS count FROM pragma_table_info('invoice') WHERE name = 'currency'",
        )
        .await,
        0
    );
}

pub use sea_orm_migration::prelude::*;

mod m20240420_1_wallet_table;
mod m20240420_2_ln_address_table;
mod m20240420_3_invoice_table;
mod m20240420_4_payment_table;
mod m20241005_5_ln_address_nostr;
mod m20241009_6_api_key_table;
mod m20241028_135908_permissions_as_json;
mod m20250106_141600_config_table;
mod m20251021_162217_convert_timestamptz_to_timestamp;
mod m20251224_162538_btc_address_table;
mod m20251224_162542_btc_output_table;
mod m20251224_162546_btc_fields_to_invoice;
mod m20251224_162550_btc_fields_to_payment;
mod m20260113_222755_fix_invoice_payment_hash_unique;
mod m20260609_143600_wallet_balance_table;
mod m20260609_143601_backfill_wallet_balances;
mod m20260704_000001_account_table;
mod m20260704_000002_auth_identity_table;
mod m20260704_000004_account_preference_table;
mod m20260704_000005_asset_table;
mod m20260704_000006_api_key_account_id;
mod m20260704_000007_backfill_oauth2_accounts;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240420_1_wallet_table::Migration),
            Box::new(m20240420_2_ln_address_table::Migration),
            Box::new(m20240420_3_invoice_table::Migration),
            Box::new(m20240420_4_payment_table::Migration),
            Box::new(m20241005_5_ln_address_nostr::Migration),
            Box::new(m20241009_6_api_key_table::Migration),
            Box::new(m20241028_135908_permissions_as_json::Migration),
            Box::new(m20250106_141600_config_table::Migration),
            Box::new(m20251021_162217_convert_timestamptz_to_timestamp::Migration),
            Box::new(m20251224_162538_btc_address_table::Migration),
            Box::new(m20251224_162542_btc_output_table::Migration),
            Box::new(m20251224_162546_btc_fields_to_invoice::Migration),
            Box::new(m20251224_162550_btc_fields_to_payment::Migration),
            Box::new(m20260113_222755_fix_invoice_payment_hash_unique::Migration),
            Box::new(m20260609_143600_wallet_balance_table::Migration),
            Box::new(m20260609_143601_backfill_wallet_balances::Migration),
            Box::new(m20260704_000001_account_table::Migration),
            Box::new(m20260704_000002_auth_identity_table::Migration),
            Box::new(m20260704_000004_account_preference_table::Migration),
            Box::new(m20260704_000005_asset_table::Migration),
            Box::new(m20260704_000006_api_key_account_id::Migration),
            Box::new(m20260704_000007_backfill_oauth2_accounts::Migration),
        ]
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};

    use super::*;

    const MIGRATIONS_BEFORE_IDENTITY_ASSETS: u32 = 16;

    async fn sqlite() -> sea_orm::DatabaseConnection {
        Database::connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite")
    }

    async fn count(conn: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
        conn.query_one(Statement::from_string(DatabaseBackend::Sqlite, sql.to_string()))
            .await
            .expect("query count")
            .expect("count row")
            .try_get::<i64>("", "count")
            .expect("count value")
    }

    #[async_std::test]
    async fn identity_asset_migration_seeds_native_btc_assets() {
        let conn = sqlite().await;

        Migrator::up(&conn, None).await.expect("run migrations");

        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM asset").await, 6);
        assert_eq!(
            count(&conn, "SELECT COUNT(*) AS count FROM asset WHERE length(id) = 16").await,
            6
        );
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
                "SELECT COUNT(*) AS count FROM asset WHERE network IN ('bitcoin/testnet', 'bitcoin/testnet4')",
            )
            .await,
            2
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) AS count FROM asset WHERE network = 'bitcoin/signet' AND display_ticker = 'sBTC'",
            )
            .await,
            1
        );
    }

    #[async_std::test]
    async fn identity_asset_migration_backfills_oauth2_accounts() {
        let conn = sqlite().await;

        Migrator::up(&conn, Some(MIGRATIONS_BEFORE_IDENTITY_ASSETS))
            .await
            .expect("run migrations before identity assets");
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO wallet (id, user_id, created_at)
            VALUES ('11111111-1111-4111-8111-111111111111', 'alice', CURRENT_TIMESTAMP)
            "#
            .to_string(),
        ))
        .await
        .expect("insert legacy wallet");
        Migrator::up(&conn, None).await.expect("run identity assets migration");

        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM account").await, 1);
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) AS count FROM auth_identity WHERE provider = 'oauth2' AND subject = 'alice'",
            )
            .await,
            1
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) AS count FROM auth_identity WHERE length(id) = 16 AND length(account_id) = 16",
            )
            .await,
            1
        );
        assert_eq!(
            count(&conn, "SELECT COUNT(*) AS count FROM account WHERE permissions = '[]'").await,
            1
        );
        assert_eq!(
            count(
                &conn,
                r#"
                SELECT COUNT(*) AS count
                FROM account_preference
                WHERE account_id = (
                    SELECT account_id
                    FROM auth_identity
                    WHERE provider = 'oauth2'
                      AND subject = 'alice'
                )
                "#,
            )
            .await,
            1
        );
    }
}

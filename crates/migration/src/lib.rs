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
mod m20260704_000008_wallet_account_asset_schema;
mod m20260704_000009_backfill_mainnet_wallet_accounts;
mod m20260704_000010_drop_api_key_user_id_fk;

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
            Box::new(m20260704_000008_wallet_account_asset_schema::Migration),
            Box::new(m20260704_000009_backfill_mainnet_wallet_accounts::Migration),
            Box::new(m20260704_000010_drop_api_key_user_id_fk::Migration),
        ]
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};

    use super::*;

    const MIGRATIONS_BEFORE_IDENTITY_ASSETS: u32 = 16;
    const IDENTITY_ASSET_MIGRATION_COUNT: u32 = 9;

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
            VALUES (X'11111111111141118111111111111111', 'alice', CURRENT_TIMESTAMP)
            "#
            .to_string(),
        ))
        .await
        .expect("insert legacy wallet");
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO wallet_balance (wallet_id, currency, available_amount, reserved_amount, created_at)
            VALUES (X'11111111111141118111111111111111', 'Bitcoin', 12345, 678, CURRENT_TIMESTAMP)
            "#
            .to_string(),
        ))
        .await
        .expect("insert legacy wallet balance");

        Migrator::up(&conn, Some(IDENTITY_ASSET_MIGRATION_COUNT))
            .await
            .expect("run identity assets migration");

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
        assert_eq!(
            count(
                &conn,
                r#"
                SELECT COUNT(*) AS count
                FROM wallet
                JOIN asset ON wallet.asset_id = asset.id
                WHERE wallet.account_id = (
                    SELECT account_id
                    FROM auth_identity
                    WHERE provider = 'oauth2'
                      AND subject = 'alice'
                )
                  AND asset.protocol = 'bitcoin'
                  AND asset.network = 'bitcoin/mainnet'
                  AND asset.asset_ref = 'native'
                  AND wallet.available_amount = 12345
                  AND wallet.reserved_amount = 678
                "#,
            )
            .await,
            1
        );
    }

    #[async_std::test]
    async fn api_key_uses_account_id_without_a_wallet_subject() {
        let conn = sqlite().await;

        Migrator::up(&conn, None).await.expect("run migrations");

        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO account (id, permissions, created_at)
            VALUES (X'22222222222242228222222222222222', '[]', CURRENT_TIMESTAMP)
            "#
            .to_string(),
        ))
        .await
        .expect("insert account");
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO api_key (id, account_id, name, key_hash, permissions, created_at)
            VALUES (
                X'33333333333343338333333333333333',
                X'22222222222242228222222222222222',
                'ci',
                X'4444444444444444444444444444444444444444444444444444444444444444',
                '[]',
                CURRENT_TIMESTAMP
            )
            "#
            .to_string(),
        ))
        .await
        .expect("insert api key with account_id only");

        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM api_key").await, 1);
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) AS count FROM pragma_table_info('api_key') WHERE name = 'user_id'",
            )
            .await,
            0
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) AS count FROM pragma_table_info('wallet') WHERE name = 'user_id'",
            )
            .await,
            0
        );
    }
}

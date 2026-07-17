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
mod m20260704_000010_finalize_wallet_schema;
mod m20260704_000011_ln_address_account_routes;
mod m20260704_000012_drop_legacy_wallet_contract;
mod m20260710_234825_add_relationship_indexes;
mod m20260717_105719_persist_lnurl_success_action;
mod m20260717_170449_add_client_event_log;
mod m20260717_173942_add_webhook_delivery;

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
            Box::new(m20260704_000010_finalize_wallet_schema::Migration),
            Box::new(m20260704_000011_ln_address_account_routes::Migration),
            Box::new(m20260704_000012_drop_legacy_wallet_contract::Migration),
            Box::new(m20260710_234825_add_relationship_indexes::Migration),
            Box::new(m20260717_105719_persist_lnurl_success_action::Migration),
            Box::new(m20260717_170449_add_client_event_log::Migration),
            Box::new(m20260717_173942_add_webhook_delivery::Migration),
        ]
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};

    use super::*;

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
            count(&conn, "SELECT COUNT(*) AS count FROM asset WHERE name = 'Bitcoin'").await,
            6
        );
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
                "SELECT COUNT(*) AS count FROM asset WHERE network IN ('Testnet', 'Testnet4')",
            )
            .await,
            2
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) AS count FROM asset WHERE network = 'Signet' AND display_ticker = 'sBTC'",
            )
            .await,
            1
        );
    }

    #[async_std::test]
    async fn fresh_sqlite_schema_has_required_relationships_and_indexes() {
        let conn = sqlite().await;

        Migrator::up(&conn, None).await.expect("run migrations");

        assert_eq!(
            count(
                &conn,
                r#"
                SELECT COUNT(*) AS count
                FROM sqlite_master AS tables
                JOIN pragma_foreign_key_list(tables.name) AS fk
                WHERE tables.type = 'table'
                "#,
            )
            .await,
            19
        );
        assert_eq!(
            count(
                &conn,
                r#"
                SELECT COUNT(*) AS count
                FROM pragma_foreign_key_list('invoice')
                WHERE "table" = 'btc_output'
                  AND "from" = 'btc_output_id'
                  AND "on_delete" = 'SET NULL'
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
                FROM pragma_foreign_key_list('wallet')
                WHERE "table" = 'asset'
                  AND "from" = 'asset_id'
                  AND "on_delete" = 'RESTRICT'
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
                    'idx_api_key_account_id',
                    'idx_asset_protocol_network_ref',
                    'idx_auth_identity_account_id',
                    'idx_auth_identity_provider_subject',
                    'idx_btc_address_wallet_used',
                    'idx_btc_output_txid_output_index',
                    'idx_client_event_type_resource',
                    'idx_client_event_wallet_id',
                    'idx_invoice_btc_output_id',
                    'idx_invoice_ln_address_id',
                    'idx_invoice_wallet_created_at',
                    'idx_payment_wallet_created_at',
                    'idx_webhook_delivery_due',
                    'idx_webhook_delivery_subscription_event',
                    'idx_webhook_subscription_account_wallet',
                    'idx_webhook_subscription_wallet_url',
                    'idx_wallet_asset_id'
                  )
                "#,
            )
            .await,
            17
        );
        assert_eq!(
            count(
                &conn,
                r#"
                SELECT COUNT(*) AS count
                FROM pragma_index_list('invoice')
                WHERE name = 'idx_invoice_btc_output_id' AND "unique" = 1
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
                FROM pragma_table_info('wallet')
                WHERE name IN (
                    'account_id', 'asset_id', 'available_amount', 'reserved_amount'
                ) AND "notnull" = 1
                "#,
            )
            .await,
            4
        );
        assert_eq!(
            count(
                &conn,
                r#"
                SELECT COUNT(*) AS count
                FROM pragma_table_info('auth_identity')
                WHERE name IN ('account_id', 'provider', 'subject') AND "notnull" = 1
                "#,
            )
            .await,
            3
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) AS count FROM pragma_table_info('payment') WHERE name = 'raw_success_action'",
            )
            .await,
            1
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
    }

    #[async_std::test]
    async fn api_key_uses_account_id_without_a_wallet_subject() {
        let conn = sqlite().await;

        Migrator::up(&conn, None).await.expect("run migrations");

        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO account (id, created_at)
            VALUES (X'22222222222242228222222222222222', CURRENT_TIMESTAMP)
            "#
            .to_string(),
        ))
        .await
        .expect("insert account");

        assert_eq!(
            count(&conn, "SELECT COUNT(*) AS count FROM account WHERE permissions = '[]'").await,
            1
        );
        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM auth_identity").await, 0);

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

    #[async_std::test]
    async fn account_deletion_cascades_owned_resources() {
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
            INSERT INTO auth_identity (id, account_id, provider, subject, created_at)
            VALUES (
                X'88888888888848888888888888888888',
                X'22222222222242228222222222222222',
                'oauth2',
                'auth0|alice',
                CURRENT_TIMESTAMP
            )
            "#
            .to_string(),
        ))
        .await
        .expect("insert auth identity");
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO account_preference (account_id, created_at)
            VALUES (X'22222222222242228222222222222222', CURRENT_TIMESTAMP)
            "#
            .to_string(),
        ))
        .await
        .expect("insert account preferences");
        let negative_reservation = conn
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO wallet (
                    id, account_id, asset_id, available_amount, reserved_amount, created_at
                ) VALUES (
                    X'99999999999949998999999999999999',
                    X'22222222222242228222222222222222',
                    X'00000000000040008000000000000001',
                    0,
                    -1,
                    CURRENT_TIMESTAMP
                )
                "#
                .to_string(),
            ))
            .await;
        assert!(negative_reservation.is_err());

        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO wallet (
                id, account_id, asset_id, available_amount, reserved_amount, created_at
            )
            VALUES (
                X'11111111111141118111111111111111',
                X'22222222222242228222222222222222',
                X'00000000000040008000000000000001',
                0,
                0,
                CURRENT_TIMESTAMP
            )
            "#
            .to_string(),
        ))
        .await
        .expect("insert account wallet");
        let duplicate_asset_wallet = conn
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO wallet (
                    id, account_id, asset_id, available_amount, reserved_amount, created_at
                ) VALUES (
                    X'aaaaaaaaaaaa4aaa8aaaaaaaaaaaaaaa',
                    X'22222222222242228222222222222222',
                    X'00000000000040008000000000000001',
                    0,
                    0,
                    CURRENT_TIMESTAMP
                )
                "#
                .to_string(),
            ))
            .await;
        assert!(duplicate_asset_wallet.is_err());
        let delete_asset_in_use = conn
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "DELETE FROM asset WHERE id = X'00000000000040008000000000000001'".to_string(),
            ))
            .await;
        assert!(delete_asset_in_use.is_err());

        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO account (id, permissions, created_at)
            VALUES (X'66666666666646668666666666666666', '[]', CURRENT_TIMESTAMP)
            "#
            .to_string(),
        ))
        .await
        .expect("insert second account");
        let mismatched_owner = conn
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
                INSERT INTO ln_address (
                    id, account_id, wallet_id, username, active, allows_nostr, created_at
                ) VALUES (
                    X'77777777777747778777777777777777',
                    X'66666666666646668666666666666666',
                    X'11111111111141118111111111111111',
                    'wrong-owner',
                    1,
                    0,
                    CURRENT_TIMESTAMP
                )
                "#
                .to_string(),
            ))
            .await;
        assert!(mismatched_owner.is_err());

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
        .expect("insert API key");
        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            INSERT INTO ln_address (
                id, account_id, wallet_id, username, active, allows_nostr, created_at
            )
            VALUES (
                X'55555555555545558555555555555555',
                X'22222222222242228222222222222222',
                X'11111111111141118111111111111111',
                'alice',
                1,
                0,
                CURRENT_TIMESTAMP
            )
            "#
            .to_string(),
        ))
        .await
        .expect("insert lightning address");

        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) AS count FROM pragma_table_info('wallet') WHERE name IN ('account_id', 'asset_id') AND \"notnull\" = 1",
            )
            .await,
            2
        );
        assert_eq!(
            count(
                &conn,
                r#"
                SELECT COUNT(*) AS count
                FROM pragma_foreign_key_list('wallet')
                WHERE "table" = 'account' AND "on_delete" = 'CASCADE'
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
                FROM pragma_foreign_key_list('api_key')
                WHERE "table" = 'account' AND "on_delete" = 'CASCADE'
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
                FROM pragma_foreign_key_list('ln_address')
                WHERE "table" = 'account' AND "on_delete" = 'CASCADE'
                "#,
            )
            .await,
            1
        );

        conn.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "DELETE FROM account WHERE id = X'22222222222242228222222222222222'".to_string(),
        ))
        .await
        .expect("delete account");

        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM wallet").await, 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM api_key").await, 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM ln_address").await, 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM auth_identity").await, 0);
        assert_eq!(
            count(&conn, "SELECT COUNT(*) AS count FROM account_preference").await,
            0
        );
        assert_eq!(count(&conn, "SELECT COUNT(*) AS count FROM asset").await, 6);
    }

    #[async_std::test]
    async fn drop_legacy_wallet_contract_removes_old_balance_and_currency_columns() {
        let conn = sqlite().await;

        Migrator::up(&conn, None).await.expect("run migrations");

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
}

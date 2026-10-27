use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        // Legacy API keys reference wallet.user_id. Backfill their account owners
        // before dropping that foreign key and the legacy wallet owner column.
        let db = manager.get_connection();
        let legacy_api_keys = manager.has_column("api_key", "user_id").await?;
        if legacy_api_keys {
            db.execute_unprepared(
                r#"
                UPDATE api_key
                SET account_id = auth_identity.account_id
                FROM auth_identity
                WHERE api_key.account_id IS NULL
                  AND auth_identity.provider = 'oauth2'
                  AND auth_identity.subject = api_key.user_id
                "#,
            )
            .await?;
        }

        let missing = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT COUNT(*) AS count FROM api_key WHERE account_id IS NULL".to_string(),
            ))
            .await?
            .ok_or_else(|| DbErr::Migration("API-key ownership count returned no row".to_string()))?
            .try_get::<i64>("", "count")?;
        if missing != 0 {
            return Err(DbErr::Migration(format!(
                "{missing} API keys have no account owner; refusing to finalize ownership"
            )));
        }

        db.execute_unprepared("ALTER TABLE api_key ALTER COLUMN account_id SET NOT NULL")
            .await?;
        if legacy_api_keys {
            db.execute_unprepared("ALTER TABLE api_key DROP COLUMN user_id").await?;
        }

        manager
            .get_connection()
            .execute_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                r#"
                ALTER TABLE wallet
                    ALTER COLUMN account_id SET NOT NULL,
                    ALTER COLUMN asset_id SET NOT NULL,
                    ADD CONSTRAINT fk_wallet_account
                        FOREIGN KEY (account_id) REFERENCES account(id) ON DELETE CASCADE,
                    ADD CONSTRAINT fk_wallet_asset
                        FOREIGN KEY (asset_id) REFERENCES asset(id) ON DELETE RESTRICT,
                    ADD CONSTRAINT chk_wallet_reserved_amount
                        CHECK (reserved_amount >= 0),
                    DROP COLUMN IF EXISTS user_id
                "#
                .to_string(),
            ))
            .await
            .map(|_| ())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "the account-owned wallet cutover is irreversible".to_string(),
        ))
    }
}

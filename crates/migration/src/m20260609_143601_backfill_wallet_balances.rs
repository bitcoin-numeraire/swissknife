use sea_orm::{DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
            UPDATE payment
            SET reserved_amount = amount_msat + COALESCE(fee_msat, 0)
            WHERE status = 'Pending'
              AND reserved_amount = 0
            "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            INSERT INTO wallet_balance (
                wallet_id,
                currency,
                available_amount,
                reserved_amount,
                created_at
            )
            SELECT
                wallet_assets.wallet_id,
                wallet_assets.currency,
                COALESCE(received.received_amount, 0)
                    - COALESCE(spent.spent_amount, 0)
                    - COALESCE(reserved.reserved_amount, 0) AS available_amount,
                COALESCE(reserved.reserved_amount, 0) AS reserved_amount,
                CURRENT_TIMESTAMP
            FROM (
                SELECT wallet_id, currency FROM invoice
                UNION
                SELECT wallet_id, currency FROM payment
            ) wallet_assets
            LEFT JOIN (
                SELECT
                    wallet_id,
                    currency,
                    SUM(COALESCE(amount_received_msat, 0)) AS received_amount
                FROM invoice
                WHERE payment_time IS NOT NULL
                GROUP BY wallet_id, currency
            ) received
                ON received.wallet_id = wallet_assets.wallet_id
               AND received.currency = wallet_assets.currency
            LEFT JOIN (
                SELECT
                    wallet_id,
                    currency,
                    SUM(amount_msat + COALESCE(fee_msat, 0)) AS spent_amount
                FROM payment
                WHERE status = 'Settled'
                GROUP BY wallet_id, currency
            ) spent
                ON spent.wallet_id = wallet_assets.wallet_id
               AND spent.currency = wallet_assets.currency
            LEFT JOIN (
                SELECT
                    wallet_id,
                    currency,
                    SUM(reserved_amount) AS reserved_amount
                FROM payment
                WHERE status = 'Pending'
                GROUP BY wallet_id, currency
            ) reserved
                ON reserved.wallet_id = wallet_assets.wallet_id
               AND reserved.currency = wallet_assets.currency
            "#,
        )
        .await?;

        let negative_count = match db.get_database_backend() {
            DatabaseBackend::Postgres => db
                .query_one(Statement::from_string(
                    DatabaseBackend::Postgres,
                    "SELECT COUNT(*)::BIGINT AS negative_count FROM wallet_balance WHERE available_amount < 0 OR reserved_amount < 0",
                ))
                .await?,
            backend => db
                .query_one(Statement::from_string(
                    backend,
                    "SELECT COUNT(*) AS negative_count FROM wallet_balance WHERE available_amount < 0 OR reserved_amount < 0",
                ))
                .await?,
        }
        .map(|row| row.try_get::<i64>("", "negative_count"))
        .transpose()?
        .unwrap_or(0);

        if negative_count > 0 {
            return Err(DbErr::Migration(
                "wallet balance backfill produced negative balances; manual reconciliation required".to_string(),
            ));
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("DELETE FROM wallet_balance").await?;
        db.execute_unprepared("UPDATE payment SET reserved_amount = 0").await?;

        Ok(())
    }
}

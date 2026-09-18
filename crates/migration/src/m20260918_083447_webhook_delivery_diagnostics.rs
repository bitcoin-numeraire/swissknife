use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let statements: &[&str] = if backend == DatabaseBackend::Postgres {
            &[
                "ALTER TABLE webhook_delivery ALTER COLUMN client_event_id DROP NOT NULL",
                "ALTER TABLE webhook_delivery ADD COLUMN test_payload JSONB",
                "ALTER TABLE webhook_delivery ADD CONSTRAINT chk_webhook_delivery_source CHECK ((client_event_id IS NOT NULL AND test_payload IS NULL) OR (client_event_id IS NULL AND test_payload IS NOT NULL))",
            ]
        } else {
            &[
                "CREATE TABLE webhook_delivery_new (
                    id UUID NOT NULL PRIMARY KEY,
                    subscription_id UUID NOT NULL REFERENCES webhook_subscription(id) ON DELETE CASCADE,
                    client_event_id INTEGER REFERENCES client_event(id) ON DELETE RESTRICT,
                    status VARCHAR(32) NOT NULL,
                    attempt_count INTEGER NOT NULL DEFAULT 0,
                    next_attempt_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    locked_until TIMESTAMP,
                    response_status INTEGER,
                    last_error TEXT,
                    delivered_at TIMESTAMP,
                    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP,
                    test_payload JSON,
                    CONSTRAINT chk_webhook_delivery_source CHECK (
                        (client_event_id IS NOT NULL AND test_payload IS NULL) OR
                        (client_event_id IS NULL AND test_payload IS NOT NULL))
                )",
                "INSERT INTO webhook_delivery_new (id, subscription_id, client_event_id, status, attempt_count, next_attempt_at, locked_until, response_status, last_error, delivered_at, created_at, updated_at)
                 SELECT id, subscription_id, client_event_id, status, attempt_count, next_attempt_at, locked_until, response_status, last_error, delivered_at, created_at, updated_at FROM webhook_delivery",
                "DROP TABLE webhook_delivery",
                "ALTER TABLE webhook_delivery_new RENAME TO webhook_delivery",
                "CREATE UNIQUE INDEX idx_webhook_delivery_subscription_event ON webhook_delivery(subscription_id, client_event_id)",
                "CREATE INDEX idx_webhook_delivery_due ON webhook_delivery(status, next_attempt_at, locked_until)",
            ]
        };
        let transaction = manager.get_connection().begin().await?;
        for sql in statements {
            transaction.execute_raw(Statement::from_string(backend, *sql)).await?;
        }
        transaction
            .execute_raw(Statement::from_string(
                backend,
                "CREATE INDEX idx_webhook_delivery_history ON webhook_delivery(subscription_id, created_at, id)"
                    .to_string(),
            ))
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "webhook test deliveries cannot be converted into wallet events".to_string(),
        ))
    }
}

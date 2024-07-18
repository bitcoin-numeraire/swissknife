use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            r#"
            CREATE TABLE payment (
                id UUID PRIMARY KEY DEFAULT gen_random_UUID(),
                wallet_id UUID NOT NULL,
                ln_address VARCHAR(255),
                payment_hash VARCHAR(255) UNIQUE,
                payment_preimage VARCHAR(255) UNIQUE,
                error VARCHAR,
                amount_msat BIGINT NOT NULL,
                fee_msat BIGINT,
                payment_time TIMESTAMPTZ,
                status VARCHAR NOT NULL,
                ledger VARCHAR(255) NOT NULL,
                currency VARCHAR(255) NOT NULL,
                description VARCHAR,
                metadata VARCHAR,
                success_action JSONB,
                created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
                updated_at TIMESTAMPTZ,
                CONSTRAINT fk_wallet FOREIGN KEY (wallet_id) REFERENCES wallet (id) ON DELETE CASCADE
            );

            CREATE OR REPLACE FUNCTION update_payment_timestamp() RETURNS TRIGGER AS $$
            BEGIN
                NEW.updated_at = NOW();
                RETURN NEW;
            END;
            $$ language 'plpgsql';

            CREATE TRIGGER update_payment_timestamp BEFORE UPDATE ON payment FOR EACH ROW EXECUTE PROCEDURE update_payment_timestamp();
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE payment")
            .await?;

        Ok(())
    }
}

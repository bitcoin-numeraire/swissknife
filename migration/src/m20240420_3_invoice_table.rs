use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            r#"
            CREATE TABLE invoice (
                id UUID PRIMARY KEY DEFAULT gen_random_UUID(),
                wallet_id UUID NOT NULL,
                payment_hash VARCHAR(255),
                ln_address_id UUID,
                bolt11 VARCHAR,
                ledger VARCHAR(255) NOT NULL,
                payee_pubkey VARCHAR,
                description VARCHAR,
                description_hash VARCHAR,
                amount_msat BIGINT,
                amount_received_msat BIGINT,
                currency VARCHAR(255) NOT NULL,
                payment_secret VARCHAR,
                timestamp TIMESTAMPTZ NOT NULL,
                expiry BIGINT,
                min_final_cltv_expiry_delta BIGINT,
                fee_msat BIGINT,
                payment_time TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
                updated_at TIMESTAMPTZ,
                expires_at TIMESTAMPTZ,
                CONSTRAINT fk_wallet FOREIGN KEY (wallet_id) REFERENCES wallet (id) ON DELETE CASCADE,
                CONSTRAINT fk_ln_address FOREIGN KEY (ln_address_id) REFERENCES ln_address (id) ON DELETE SET NULL
            );

            CREATE OR REPLACE FUNCTION update_invoice_timestamp() RETURNS TRIGGER AS $$
            BEGIN
                NEW.updated_at = NOW();
                RETURN NEW;
            END;
            $$ language 'plpgsql';

            CREATE UNIQUE INDEX UNIQUE_payment_hash ON invoice(payment_hash) WHERE payment_hash IS NOT NULL;
            CREATE UNIQUE INDEX UNIQUE_bolt11 ON invoice(bolt11) WHERE bolt11 IS NOT NULL;

            CREATE TRIGGER update_invoice_timestamp BEFORE UPDATE ON invoice FOR EACH ROW EXECUTE PROCEDURE update_invoice_timestamp();
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE invoice")
            .await?;

        Ok(())
    }
}

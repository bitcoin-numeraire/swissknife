use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Use `execute_unprepared` if the SQL statement doesn't have value bindings
        db.execute_unprepared(
            "CREATE OR REPLACE FUNCTION update_invoice_timestamp() RETURNS TRIGGER AS $$ BEGIN NEW.updated_at = NOW();
            RETURN NEW;
            END;
            $$ language 'plpgsql';
            CREATE TABLE invoice (
                id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id varchar(255) NOT NULL,
                invoice_type varchar(255) NOT NULL,
                payment_hash varchar(255),
                ln_address uuid,
                bolt11 varchar,
                network varchar NOT NULL,
                payee_pubkey varchar,
                description varchar,
                description_hash varchar,
                amount_msat bigint,
                payment_secret varchar,
                timestamp timestamptz NOT NULL,
                expiry bigint,
                min_final_cltv_expiry_delta bigint,
                fee_msat bigint,
                payment_time timestamptz,
                label uuid,
                created_at timestamptz NOT NULL DEFAULT current_timestamp,
                updated_at timestamptz,
                expires_at timestamptz,
                CONSTRAINT fk_ln_address FOREIGN KEY (ln_address)
                REFERENCES ln_address (id)
                ON DELETE SET NULL
            );
            CREATE UNIQUE INDEX unique_payment_hash ON invoice(payment_hash) WHERE payment_hash IS NOT NULL;
            CREATE UNIQUE INDEX unique_bolt11 ON invoice(bolt11) WHERE bolt11 IS NOT NULL;

            CREATE TRIGGER update_invoice_timestamp BEFORE
            UPDATE ON invoice FOR EACH ROW EXECUTE PROCEDURE update_invoice_timestamp();",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE `invoice`")
            .await?;

        Ok(())
    }
}

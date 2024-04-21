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
            CREATE TABLE lightning_invoice (
                id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
                lightning_address varchar(255),
                bolt11 varchar unique NOT NULL,
                network varchar NOT NULL,
                payee_pubkey varchar NOT NULL,
                payment_hash varchar unique NOT NULL,
                description varchar,
                description_hash varchar,
                amount_msat bigint NOT NULL,
                payment_secret bytea NOT NULL,
                timestamp bigint NOT NULL,
                expiry bigint NOT NULL,
                min_final_cltv_expiry_delta bigint NOT NULL,
                status varchar NOT NULL,
                fee_msat bigint,
                payment_time bigint,
                created_at timestamptz NOT NULL DEFAULT current_timestamp,
                updated_at timestamptz
            );
            CREATE TRIGGER update_lightning_invoice_timestamp BEFORE
            UPDATE ON lightning_invoice FOR EACH ROW EXECUTE PROCEDURE update_invoice_timestamp();
            ALTER TABLE lightning_invoice
            ADD CONSTRAINT fk_lightning_address FOREIGN KEY (lightning_address) REFERENCES lightning_address(username) ON DELETE CASCADE ON UPDATE CASCADE;",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE `lightning_invoice`")
            .await?;

        Ok(())
    }
}

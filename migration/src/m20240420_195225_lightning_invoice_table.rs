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
                payment_hash varchar(255) PRIMARY KEY,
                user_id varchar(255) NOT NULL,
                lightning_address varchar(255),
                bolt11 varchar unique NOT NULL,
                network varchar NOT NULL,
                payee_pubkey varchar NOT NULL,
                description varchar,
                description_hash varchar,
                amount_msat bigint,
                payment_secret bytea NOT NULL,
                timestamp bigint NOT NULL,
                expiry bigint NOT NULL,
                min_final_cltv_expiry_delta bigint NOT NULL,
                fee_msat bigint,
                payment_time bigint,
                details jsonb,
                created_at timestamptz NOT NULL DEFAULT current_timestamp,
                updated_at timestamptz
            );
            CREATE TRIGGER update_lightning_invoice_timestamp BEFORE
            UPDATE ON lightning_invoice FOR EACH ROW EXECUTE PROCEDURE update_invoice_timestamp();",
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

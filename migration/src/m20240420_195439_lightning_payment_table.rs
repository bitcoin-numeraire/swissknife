use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Use `execute_unprepared` if the SQL statement doesn't have value bindings
        db.execute_unprepared(
            "CREATE OR REPLACE FUNCTION update_payment_timestamp() RETURNS TRIGGER AS $$ BEGIN NEW.updated_at = NOW();
            RETURN NEW;
            END;
            $$ language 'plpgsql';
            CREATE TABLE lightning_payment (
                id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
                lightning_address varchar(255),
                payment_hash varchar,
                error varchar,
                amount_msat bigint NOT NULL,
                fee_msat bigint,
                payment_time bigint,
                status varchar NOT NULL,
                description varchar,
                metadata varchar,
                created_at timestamptz NOT NULL DEFAULT current_timestamp,
                updated_at timestamptz
            );
            CREATE TRIGGER update_lightning_payment_timestamp BEFORE
            UPDATE ON lightning_payment FOR EACH ROW EXECUTE PROCEDURE update_payment_timestamp();
            ALTER TABLE lightning_payment
            ADD CONSTRAINT fk_lightning_address FOREIGN KEY (lightning_address) REFERENCES lightning_address(username) ON DELETE CASCADE ON UPDATE CASCADE;",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE `lightning_payment`")
            .await?;

        Ok(())
    }
}

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261028_152505_payment_fee_authorization"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // A historical reservation was a buffer, not a recorded fee limit.
        // Keep old authorizations unknown; never infer them from that buffer.
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE payment ADD COLUMN authorized_fee_msat BIGINT NULL CHECK (authorized_fee_msat >= 0)",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE payment DROP COLUMN authorized_fee_msat")
            .await?;
        Ok(())
    }
}

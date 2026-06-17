use sea_orm::DatabaseBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite already treats NULLs as distinct in unique indexes, so no changes needed
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        let db = manager.get_connection();

        // Drop existing constraints/indexes (IF EXISTS prevents transaction abort)
        db.execute_unprepared(r#"ALTER TABLE invoice DROP CONSTRAINT IF EXISTS "UNIQUE_payment_hash""#)
            .await?;

        db.execute_unprepared(r#"DROP INDEX IF EXISTS "UNIQUE_payment_hash""#)
            .await?;

        db.execute_unprepared(r#"ALTER TABLE invoice DROP CONSTRAINT IF EXISTS "UNIQUE_bolt11""#)
            .await?;

        db.execute_unprepared(r#"DROP INDEX IF EXISTS "UNIQUE_bolt11""#).await?;

        // Some production databases also carry lowercase partial indexes created
        // out-of-band (never by a migration). Drop them so the schema converges to
        // the migration-defined indexes below instead of coexisting redundantly.
        db.execute_unprepared(r#"DROP INDEX IF EXISTS "unique_payment_hash""#)
            .await?;

        db.execute_unprepared(r#"DROP INDEX IF EXISTS "unique_bolt11""#).await?;

        // Recreate as unique indexes without NULLS NOT DISTINCT
        // This allows multiple NULL values (needed for onchain invoices)
        db.execute_unprepared(r#"CREATE UNIQUE INDEX "UNIQUE_payment_hash" ON invoice (payment_hash)"#)
            .await?;

        db.execute_unprepared(r#"CREATE UNIQUE INDEX "UNIQUE_bolt11" ON invoice (bolt11)"#)
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return Ok(());
        }

        let db = manager.get_connection();

        db.execute_unprepared(r#"DROP INDEX IF EXISTS "UNIQUE_payment_hash""#)
            .await?;

        db.execute_unprepared(r#"DROP INDEX IF EXISTS "UNIQUE_bolt11""#).await?;

        // Recreate with NULLS NOT DISTINCT (original behavior)
        db.execute_unprepared(
            r#"CREATE UNIQUE INDEX "UNIQUE_payment_hash" ON invoice (payment_hash) NULLS NOT DISTINCT"#,
        )
        .await?;

        db.execute_unprepared(r#"CREATE UNIQUE INDEX "UNIQUE_bolt11" ON invoice (bolt11) NULLS NOT DISTINCT"#)
            .await?;

        Ok(())
    }
}

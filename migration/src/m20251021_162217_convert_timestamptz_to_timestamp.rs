use sea_orm::DatabaseBackend;
use sea_orm_migration::{prelude::*};


#[derive(DeriveMigrationName)]
pub struct Migration;

// This migration converts TIMESTAMPTZ columns to TIMESTAMP (without timezone) for PostgreSQL.
// This is needed for production databases that were created with the old migrations.
// SQLite doesn't need this migration as it doesn't have a timezone distinction.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db_backend = manager.get_database_backend();

        if db_backend == DatabaseBackend::Postgres {
            let db = manager.get_connection();
            
            // Convert Wallet table columns
            db.execute_unprepared(
                "ALTER TABLE wallet 
                 ALTER COLUMN created_at TYPE timestamp USING created_at AT TIME ZONE 'UTC',
                 ALTER COLUMN updated_at TYPE timestamp USING updated_at AT TIME ZONE 'UTC';"
            ).await?;

            // Convert LnAddress table columns
            db.execute_unprepared(
                "ALTER TABLE ln_address 
                 ALTER COLUMN created_at TYPE timestamp USING created_at AT TIME ZONE 'UTC',
                 ALTER COLUMN updated_at TYPE timestamp USING updated_at AT TIME ZONE 'UTC';"
            ).await?;

            // Convert Invoice table columns
            db.execute_unprepared(
                "ALTER TABLE invoice 
                 ALTER COLUMN timestamp TYPE timestamp USING timestamp AT TIME ZONE 'UTC',
                 ALTER COLUMN payment_time TYPE timestamp USING payment_time AT TIME ZONE 'UTC',
                 ALTER COLUMN created_at TYPE timestamp USING created_at AT TIME ZONE 'UTC',
                 ALTER COLUMN updated_at TYPE timestamp USING updated_at AT TIME ZONE 'UTC',
                 ALTER COLUMN expires_at TYPE timestamp USING expires_at AT TIME ZONE 'UTC';"
            ).await?;

            // Convert Payment table columns
            db.execute_unprepared(
                "ALTER TABLE payment 
                 ALTER COLUMN payment_time TYPE timestamp USING payment_time AT TIME ZONE 'UTC',
                 ALTER COLUMN created_at TYPE timestamp USING created_at AT TIME ZONE 'UTC',
                 ALTER COLUMN updated_at TYPE timestamp USING updated_at AT TIME ZONE 'UTC';"
            ).await?;

            // Convert ApiKey table columns
            db.execute_unprepared(
                "ALTER TABLE api_key 
                 ALTER COLUMN created_at TYPE timestamp USING created_at AT TIME ZONE 'UTC',
                 ALTER COLUMN expires_at TYPE timestamp USING expires_at AT TIME ZONE 'UTC';"
            ).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db_backend = manager.get_database_backend();

        if db_backend == DatabaseBackend::Postgres {
            let db = manager.get_connection();
            
            // Revert Wallet table columns back to TIMESTAMPTZ
            db.execute_unprepared(
                "ALTER TABLE wallet 
                 ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC',
                 ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'UTC';"
            ).await?;

            // Revert LnAddress table columns
            db.execute_unprepared(
                "ALTER TABLE ln_address 
                 ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC',
                 ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'UTC';"
            ).await?;

            // Revert Invoice table columns
            db.execute_unprepared(
                "ALTER TABLE invoice 
                 ALTER COLUMN timestamp TYPE timestamptz USING timestamp AT TIME ZONE 'UTC',
                 ALTER COLUMN payment_time TYPE timestamptz USING payment_time AT TIME ZONE 'UTC',
                 ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC',
                 ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'UTC',
                 ALTER COLUMN expires_at TYPE timestamptz USING expires_at AT TIME ZONE 'UTC';"
            ).await?;

            // Revert Payment table columns
            db.execute_unprepared(
                "ALTER TABLE payment 
                 ALTER COLUMN payment_time TYPE timestamptz USING payment_time AT TIME ZONE 'UTC',
                 ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC',
                 ALTER COLUMN updated_at TYPE timestamptz USING updated_at AT TIME ZONE 'UTC';"
            ).await?;

            // Revert ApiKey table columns
            db.execute_unprepared(
                "ALTER TABLE api_key 
                 ALTER COLUMN created_at TYPE timestamptz USING created_at AT TIME ZONE 'UTC',
                 ALTER COLUMN expires_at TYPE timestamptz USING expires_at AT TIME ZONE 'UTC';"
            ).await?;
        }

        Ok(())
    }
}

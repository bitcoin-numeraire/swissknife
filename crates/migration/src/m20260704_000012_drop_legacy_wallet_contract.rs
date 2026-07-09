use sea_orm::{ConnectionTrait, DatabaseBackend};
use sea_orm_migration::prelude::*;

use crate::{
    m20240420_3_invoice_table::Invoice, m20240420_4_payment_table::Payment,
    m20260609_143600_wallet_balance_table::WalletBalance,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_connection().get_database_backend() == DatabaseBackend::Sqlite {
            return drop_sqlite(manager.get_connection()).await;
        }

        manager
            .drop_table(Table::drop().table(WalletBalance::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Payment::Table)
                    .drop_column(Payment::Currency)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Invoice::Table)
                    .drop_column(Invoice::Currency)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "dropping the legacy wallet contract is irreversible".to_string(),
        ))
    }
}

async fn drop_sqlite(db: &dyn ConnectionTrait) -> Result<(), DbErr> {
    db.execute_unprepared("PRAGMA foreign_keys = OFF").await?;
    db.execute_unprepared("DROP TABLE wallet_balance").await?;
    db.execute_unprepared("ALTER TABLE payment DROP COLUMN currency")
        .await?;
    db.execute_unprepared("ALTER TABLE invoice DROP COLUMN currency")
        .await?;
    db.execute_unprepared("PRAGMA foreign_keys = ON").await?;

    Ok(())
}

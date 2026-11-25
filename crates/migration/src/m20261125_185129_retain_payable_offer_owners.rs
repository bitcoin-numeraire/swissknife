use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, DatabaseBackend, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let tx = manager.get_connection().begin().await?;
        let schema = SchemaManager::new(&tx);
        schema
            .alter_table(
                Table::alter()
                    .table(Alias::new("wallet_offer"))
                    .add_column(
                        ColumnDef::new(Alias::new("deletion_ready"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        // A cascading account or wallet delete reaches the same trigger, including bulk operations.
        match tx.get_database_backend() {
            DatabaseBackend::Postgres => {
                tx.execute_unprepared(
                    "CREATE FUNCTION retain_payable_offer() RETURNS trigger LANGUAGE plpgsql AS $$
                    BEGIN RAISE EXCEPTION 'swissknife_offer_still_payable' USING ERRCODE = '23503'; END $$",
                )
                .await?;
                tx.execute_unprepared(
                    "CREATE TRIGGER retain_payable_offer BEFORE DELETE ON wallet_offer
                    FOR EACH ROW WHEN (NOT OLD.deletion_ready) EXECUTE FUNCTION retain_payable_offer()",
                )
                .await?;
            }
            DatabaseBackend::Sqlite => {
                tx.execute_unprepared(
                    "CREATE TRIGGER retain_payable_offer BEFORE DELETE ON wallet_offer
                    WHEN NOT OLD.deletion_ready BEGIN SELECT RAISE(ABORT, 'swissknife_offer_still_payable'); END",
                )
                .await?;
            }
            other => return Err(DbErr::Custom(format!("Unsupported DB backend {other:?}"))),
        }
        tx.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let tx = manager.get_connection().begin().await?;
        if tx.get_database_backend() == DatabaseBackend::Postgres {
            tx.execute_unprepared("DROP TRIGGER retain_payable_offer ON wallet_offer")
                .await?;
            tx.execute_unprepared("DROP FUNCTION retain_payable_offer()").await?;
        } else {
            tx.execute_unprepared("DROP TRIGGER retain_payable_offer").await?;
        }
        SchemaManager::new(&tx)
            .alter_table(
                Table::alter()
                    .table(Alias::new("wallet_offer"))
                    .drop_column(Alias::new("deletion_ready"))
                    .to_owned(),
            )
            .await?;
        tx.commit().await
    }
}

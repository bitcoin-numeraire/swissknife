use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("wallet_identity"))
                    .col(ColumnDef::new(Alias::new("wallet_id")).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Alias::new("recovery_pubkey")).string_len(64).not_null())
                    .col(ColumnDef::new(Alias::new("nostr_pubkey")).string_len(64).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("wallet_identity"), Alias::new("wallet_id"))
                            .to(Alias::new("wallet"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("wallet_identity")).to_owned())
            .await
    }
}

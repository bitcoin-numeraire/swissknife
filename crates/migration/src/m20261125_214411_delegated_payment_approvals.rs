use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Approval {
    #[sea_orm(iden = "agent_payment_approval")]
    Table,
    Id,
    WalletId,
    AccountId,
    AgentKeyId,
    IdempotencyKey,
    Fingerprint,
    Intent,
    State,
    CreatedAt,
    ExpiresAt,
    DecidedAt,
    DecidedBy,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        use Approval as A;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("spending_policy"))
                    .add_column(ColumnDef::new(Alias::new("approval_above_msat")).big_integer())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(A::Table)
                    .col(ColumnDef::new(A::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(A::WalletId).uuid().not_null())
                    .col(ColumnDef::new(A::AccountId).uuid().not_null())
                    .col(ColumnDef::new(A::AgentKeyId).uuid().not_null())
                    .col(ColumnDef::new(A::IdempotencyKey).uuid().not_null())
                    .col(ColumnDef::new(A::Fingerprint).string().not_null())
                    .col(ColumnDef::new(A::Intent).json_binary().not_null())
                    .col(ColumnDef::new(A::State).string().not_null())
                    .col(ColumnDef::new(A::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(A::ExpiresAt).date_time().not_null())
                    .col(ColumnDef::new(A::DecidedAt).date_time())
                    .col(ColumnDef::new(A::DecidedBy).uuid())
                    .foreign_key(
                        ForeignKey::create()
                            .from(A::Table, A::WalletId)
                            .to(Alias::new("wallet"), Alias::new("id"))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_approval_wallet_request")
                    .table(A::Table)
                    .col(A::WalletId)
                    .col(A::IdempotencyKey)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_approval_account_queue")
                    .table(A::Table)
                    .col(A::AccountId)
                    .col(A::State)
                    .col(A::ExpiresAt)
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Approval::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("spending_policy"))
                    .drop_column(Alias::new("approval_above_msat"))
                    .to_owned(),
            )
            .await
    }
}

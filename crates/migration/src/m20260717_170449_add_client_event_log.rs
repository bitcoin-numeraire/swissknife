use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240420_1_wallet_table::Wallet;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ClientEvent::Table)
                    .if_not_exists()
                    .col(pk_auto(ClientEvent::Id))
                    .col(uuid(ClientEvent::WalletId))
                    .col(string_len(ClientEvent::EventType, 64))
                    .col(uuid(ClientEvent::ResourceId))
                    .col(json_binary(ClientEvent::Payload))
                    .col(timestamp(ClientEvent::CreatedAt).default(Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_client_event_wallet")
                            .from(ClientEvent::Table, ClientEvent::WalletId)
                            .to(Wallet::Table, Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_client_event_wallet_id")
                    .table(ClientEvent::Table)
                    .col(ClientEvent::WalletId)
                    .col(ClientEvent::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_client_event_type_resource")
                    .table(ClientEvent::Table)
                    .col(ClientEvent::EventType)
                    .col(ClientEvent::ResourceId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ClientEvent::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum ClientEvent {
    Table,
    Id,
    WalletId,
    EventType,
    ResourceId,
    Payload,
    CreatedAt,
}

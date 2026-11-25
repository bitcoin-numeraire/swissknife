use sea_orm_migration::{prelude::*, schema::*};

use crate::{m20260704_000001_account_table::Account, m20260717_170449_add_client_event_log::ClientEvent};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AccountNotification::Table)
                    .col(integer(AccountNotification::SourceEventId).primary_key())
                    .col(uuid(AccountNotification::AccountId))
                    .col(boolean(AccountNotification::IsRead).default(false))
                    .col(boolean(AccountNotification::Archived).default(false))
                    .foreign_key(
                        ForeignKey::create()
                            .from(AccountNotification::Table, AccountNotification::SourceEventId)
                            .to(ClientEvent::Table, ClientEvent::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(AccountNotification::Table, AccountNotification::AccountId)
                            .to(Account::Table, Account::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_account_notification_inbox")
                    .table(AccountNotification::Table)
                    .col(AccountNotification::AccountId)
                    .col(AccountNotification::Archived)
                    .col(AccountNotification::SourceEventId)
                    .to_owned(),
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO account_notification (source_event_id, account_id)
             SELECT e.id, w.account_id FROM client_event e JOIN wallet w ON w.id = e.wallet_id
             WHERE e.event_type IN ('invoice.paid', 'payment.settled', 'payment.failed')",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AccountNotification::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AccountNotification {
    Table,
    SourceEventId,
    AccountId,
    IsRead,
    Archived,
}

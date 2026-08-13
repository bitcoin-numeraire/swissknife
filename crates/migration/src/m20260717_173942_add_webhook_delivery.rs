use sea_orm_migration::{prelude::*, schema::*};

use crate::{
    m20240420_1_wallet_table::Wallet, m20260704_000001_account_table::Account,
    m20260717_170449_add_client_event_log::ClientEvent,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebhookSubscription::Table)
                    .if_not_exists()
                    .col(uuid(WebhookSubscription::Id).primary_key())
                    .col(uuid(WebhookSubscription::AccountId))
                    .col(uuid(WebhookSubscription::WalletId))
                    .col(text(WebhookSubscription::Url))
                    .col(json_binary(WebhookSubscription::EventTypes))
                    .col(text(WebhookSubscription::SigningSecret))
                    .col(boolean(WebhookSubscription::Active).default(true))
                    .col(integer(WebhookSubscription::LastEventId).default(0))
                    .col(timestamp(WebhookSubscription::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(WebhookSubscription::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_subscription_account")
                            .from(WebhookSubscription::Table, WebhookSubscription::AccountId)
                            .to(Account::Table, Account::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_subscription_wallet")
                            .from_tbl(WebhookSubscription::Table)
                            .from_col(WebhookSubscription::AccountId)
                            .from_col(WebhookSubscription::WalletId)
                            .to_tbl(Wallet::Table)
                            .to_col(Wallet::AccountId)
                            .to_col(Wallet::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_webhook_subscription_account_wallet")
                    .table(WebhookSubscription::Table)
                    .col(WebhookSubscription::AccountId)
                    .col(WebhookSubscription::WalletId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_webhook_subscription_wallet_url")
                    .table(WebhookSubscription::Table)
                    .col(WebhookSubscription::WalletId)
                    .col(WebhookSubscription::Url)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WebhookDelivery::Table)
                    .if_not_exists()
                    .col(uuid(WebhookDelivery::Id).primary_key())
                    .col(uuid(WebhookDelivery::SubscriptionId))
                    .col(integer(WebhookDelivery::ClientEventId))
                    .col(string_len(WebhookDelivery::Status, 32))
                    .col(integer(WebhookDelivery::AttemptCount).default(0))
                    .col(timestamp(WebhookDelivery::NextAttemptAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(WebhookDelivery::LockedUntil))
                    .col(integer_null(WebhookDelivery::ResponseStatus))
                    .col(text_null(WebhookDelivery::LastError))
                    .col(timestamp_null(WebhookDelivery::DeliveredAt))
                    .col(timestamp(WebhookDelivery::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(WebhookDelivery::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_delivery_subscription")
                            .from(WebhookDelivery::Table, WebhookDelivery::SubscriptionId)
                            .to(WebhookSubscription::Table, WebhookSubscription::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_delivery_client_event")
                            .from(WebhookDelivery::Table, WebhookDelivery::ClientEventId)
                            .to(ClientEvent::Table, ClientEvent::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_webhook_delivery_subscription_event")
                    .table(WebhookDelivery::Table)
                    .col(WebhookDelivery::SubscriptionId)
                    .col(WebhookDelivery::ClientEventId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_webhook_delivery_due")
                    .table(WebhookDelivery::Table)
                    .col(WebhookDelivery::Status)
                    .col(WebhookDelivery::NextAttemptAt)
                    .col(WebhookDelivery::LockedUntil)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WebhookDelivery::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WebhookSubscription::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum WebhookSubscription {
    Table,
    Id,
    AccountId,
    WalletId,
    Url,
    EventTypes,
    SigningSecret,
    Active,
    LastEventId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
pub(crate) enum WebhookDelivery {
    Table,
    Id,
    SubscriptionId,
    ClientEventId,
    Status,
    AttemptCount,
    NextAttemptAt,
    LockedUntil,
    ResponseStatus,
    LastError,
    DeliveredAt,
    CreatedAt,
    UpdatedAt,
}

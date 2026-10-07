use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    sea_query::{Expr, OnConflict, Query},
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DatabaseTransaction, EntityTrait, ExprTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::{
        event::{ClientEvent, ClientEventRepository, ClientEventType},
        invoice::Invoice,
        payment::{Payment, PaymentStatus},
    },
    infra::database::sea_orm::models::{
        client_event::{ActiveModel, Column},
        config,
        prelude::{
            ClientEvent as ClientEventEntity, Config as ConfigEntity, Wallet as WalletEntity,
            WebhookDelivery as WebhookDeliveryEntity, WebhookSubscription as WebhookSubscriptionEntity,
        },
        wallet, webhook_delivery, webhook_subscription,
    },
};

const PRUNED_THROUGH_KEY: &str = "client_event_pruned_through_id";

/// A snapshot to append; the database assigns its event ID and creation time.
#[derive(Clone, Debug)]
pub(in crate::infra::database::sea_orm) struct NewClientEvent {
    pub event_type: ClientEventType,
    pub wallet_id: Uuid,
    pub resource_id: Uuid,
    pub data: Value,
}

impl NewClientEvent {
    pub fn invoice_pending(invoice: &Invoice) -> Self {
        Self {
            event_type: ClientEventType::InvoicePending,
            wallet_id: invoice.wallet_id,
            resource_id: invoice.id,
            data: serde_json::to_value(invoice).expect("should serialize successfully by assertion"),
        }
    }

    pub fn invoice_paid(invoice: &Invoice) -> Self {
        Self {
            event_type: ClientEventType::InvoicePaid,
            wallet_id: invoice.wallet_id,
            resource_id: invoice.id,
            data: serde_json::to_value(invoice).expect("should serialize successfully by assertion"),
        }
    }

    pub fn payment(payment: &Payment) -> Self {
        let event_type = match payment.status {
            PaymentStatus::Settled => ClientEventType::PaymentSettled,
            PaymentStatus::Failed => ClientEventType::PaymentFailed,
            _ => unreachable!("payment should be terminal by assertion"),
        };

        Self {
            event_type,
            wallet_id: payment.wallet_id,
            resource_id: payment.id,
            data: serde_json::to_value(payment).expect("should serialize successfully by assertion"),
        }
    }
}

// Hold this write lock until commit so sequence allocation follows commit order.
// PostgreSQL sequences alone can expose a higher ID before a lower ID commits.
pub(crate) async fn lock_client_event_log(txn: &DatabaseTransaction) -> Result<(), DatabaseError> {
    ConfigEntity::insert(config::ActiveModel {
        key: Set("client_event_append_lock".to_string()),
        value: Set(None),
    })
    .on_conflict(
        OnConflict::column(config::Column::Key)
            .update_column(config::Column::Value)
            .to_owned(),
    )
    .exec_without_returning(txn)
    .await
    .map_err(|error| DatabaseError::Update(error.to_string()))?;
    Ok(())
}

#[derive(Clone)]
pub struct SeaOrmClientEventRepository<C = DatabaseConnection> {
    db: C,
}

impl<C> SeaOrmClientEventRepository<C> {
    pub fn new(db: C) -> Self {
        Self { db }
    }
}

impl SeaOrmClientEventRepository<&DatabaseTransaction> {
    pub(in crate::infra::database::sea_orm) async fn append_event(
        &self,
        event: NewClientEvent,
    ) -> Result<(), DatabaseError> {
        lock_client_event_log(self.db).await?;
        ClientEventEntity::insert(ActiveModel {
            wallet_id: Set(event.wallet_id),
            event_type: Set(event.event_type.to_string()),
            resource_id: Set(event.resource_id),
            payload: Set(event.data),
            ..Default::default()
        })
        .on_conflict(
            OnConflict::columns([Column::EventType, Column::ResourceId])
                .do_nothing()
                .to_owned(),
        )
        .exec_without_returning(self.db)
        .await
        .map_err(|error| DatabaseError::Insert(error.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl ClientEventRepository for SeaOrmClientEventRepository<DatabaseConnection> {
    async fn latest_id(&self, account_id: Uuid) -> Result<Option<i32>, DatabaseError> {
        Ok(ClientEventEntity::find()
            .inner_join(WalletEntity)
            .filter(wallet::Column::AccountId.eq(account_id))
            .order_by_desc(Column::Id)
            .one(&self.db)
            .await
            .map_err(|error| DatabaseError::FindOne(error.to_string()))?
            .map(|model| model.id))
    }

    async fn find_after(&self, account_id: Uuid, after_id: i32, limit: u64) -> Result<Vec<ClientEvent>, DatabaseError> {
        Ok(ClientEventEntity::find()
            .inner_join(WalletEntity)
            .filter(wallet::Column::AccountId.eq(account_id))
            .filter(Column::Id.gt(after_id))
            .order_by_asc(Column::Id)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|error| DatabaseError::FindMany(error.to_string()))?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn pruned_through(&self) -> Result<i32, DatabaseError> {
        let Some(model) = ConfigEntity::find_by_id(PRUNED_THROUGH_KEY)
            .one(&self.db)
            .await
            .map_err(|error| DatabaseError::FindOne(error.to_string()))?
        else {
            return Ok(0);
        };
        let value = model
            .value
            .and_then(|value| value.as_i64())
            .ok_or_else(|| DatabaseError::FindOne("client event prune watermark is malformed".to_string()))?;

        i32::try_from(value)
            .map_err(|error| DatabaseError::FindOne(format!("client event prune watermark is invalid: {error}")))
    }

    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64, DatabaseError> {
        // Ensure every replica has a row it can lock. Serializing retention on
        // this row prevents a slower cleanup pass from moving the watermark
        // backwards after a newer pass has committed.
        ConfigEntity::insert(config::ActiveModel {
            key: Set(PRUNED_THROUGH_KEY.to_string()),
            value: Set(Some(serde_json::json!(0))),
        })
        .on_conflict(OnConflict::column(config::Column::Key).do_nothing().to_owned())
        .exec_without_returning(&self.db)
        .await
        .map_err(|error| DatabaseError::Insert(error.to_string()))?;

        let txn = self
            .db
            .begin()
            .await
            .map_err(|error| DatabaseError::Transaction(error.to_string()))?;
        lock_client_event_log(&txn).await?;
        let watermark = ConfigEntity::find_by_id(PRUNED_THROUGH_KEY)
            .lock_exclusive()
            .one(&txn)
            .await
            .map_err(|error| DatabaseError::FindOne(error.to_string()))?
            .ok_or_else(|| DatabaseError::FindOne("client event prune watermark is missing".to_string()))?;
        let unconsumed = Query::select()
            .expr(Expr::value(1))
            .from(WebhookSubscriptionEntity)
            .and_where(Expr::col((WebhookSubscriptionEntity, webhook_subscription::Column::Active)).eq(true))
            .and_where(
                Expr::col((WebhookSubscriptionEntity, webhook_subscription::Column::WalletId))
                    .equals((ClientEventEntity, Column::WalletId)),
            )
            .and_where(
                Expr::col((WebhookSubscriptionEntity, webhook_subscription::Column::LastEventId))
                    .lt(Expr::col((ClientEventEntity, Column::Id))),
            )
            .to_owned();
        let pending = Query::select()
            .expr(Expr::value(1))
            .from(WebhookDeliveryEntity)
            .and_where(Expr::col((WebhookDeliveryEntity, webhook_delivery::Column::Status)).eq("Pending"))
            .and_where(
                Expr::col((WebhookDeliveryEntity, webhook_delivery::Column::ClientEventId))
                    .equals((ClientEventEntity, Column::Id)),
            )
            .to_owned();
        let first_retained_id = ClientEventEntity::find()
            .filter(
                Condition::any()
                    .add(Column::CreatedAt.gte(cutoff.naive_utc()))
                    .add(Expr::exists(unconsumed))
                    .add(Expr::exists(pending)),
            )
            .order_by_asc(Column::Id)
            .one(&txn)
            .await
            .map_err(|error| DatabaseError::FindOne(error.to_string()))?
            .map(|event| event.id);
        let mut expired = ClientEventEntity::find().filter(Column::CreatedAt.lt(cutoff.naive_utc()));
        if let Some(first_retained_id) = first_retained_id {
            // Cursors are global monotonic IDs, so only remove a contiguous
            // expired prefix. If timestamps are ever out of order, an older
            // later event waits for the retained event before it to expire.
            expired = expired.filter(Column::Id.lt(first_retained_id));
        }
        let Some(last_expired) = expired
            .order_by_desc(Column::Id)
            .one(&txn)
            .await
            .map_err(|error| DatabaseError::FindOne(error.to_string()))?
        else {
            txn.commit()
                .await
                .map_err(|error| DatabaseError::Transaction(error.to_string()))?;
            return Ok(0);
        };

        WebhookDeliveryEntity::delete_many()
            .filter(webhook_delivery::Column::ClientEventId.lte(last_expired.id))
            .filter(webhook_delivery::Column::Status.ne("Pending"))
            .exec(&txn)
            .await
            .map_err(|error| DatabaseError::Delete(error.to_string()))?;

        let deleted = ClientEventEntity::delete_many()
            .filter(Column::Id.lte(last_expired.id))
            .exec(&txn)
            .await
            .map_err(|error| DatabaseError::Delete(error.to_string()))?;

        let previous_id = watermark
            .value
            .as_ref()
            .and_then(serde_json::Value::as_i64)
            .and_then(|value| i32::try_from(value).ok())
            .ok_or_else(|| DatabaseError::FindOne("client event prune watermark is malformed".to_string()))?;
        let mut watermark: config::ActiveModel = watermark.into();
        watermark.value = Set(Some(serde_json::json!(std::cmp::max(previous_id, last_expired.id))));
        watermark
            .update(&txn)
            .await
            .map_err(|error| DatabaseError::Update(error.to_string()))?;

        txn.commit()
            .await
            .map_err(|error| DatabaseError::Transaction(error.to_string()))?;

        Ok(deleted.rows_affected)
    }
}

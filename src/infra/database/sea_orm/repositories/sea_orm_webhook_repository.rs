use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    sea_query::{Expr, OnConflict},
    ActiveModelTrait,
    ActiveValue::Unchanged,
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::event::{
        ClaimedWebhookDelivery, ClientEvent, ClientEventType, NewWebhookSubscription, StoredWebhookSubscription,
        WebhookDelivery, WebhookDeliveryStatus, WebhookRepository,
    },
    infra::database::sea_orm::models::{
        client_event,
        prelude::{
            ClientEvent as ClientEventEntity, WebhookDelivery as WebhookDeliveryEntity,
            WebhookSubscription as WebhookSubscriptionEntity,
        },
        webhook_delivery, webhook_subscription,
    },
};

const PENDING: &str = "Pending";
const DELIVERED: &str = "Delivered";
const EXHAUSTED: &str = "Exhausted";

#[derive(Clone)]
pub struct SeaOrmWebhookRepository {
    db: DatabaseConnection,
}

impl SeaOrmWebhookRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn parse_event_types(value: serde_json::Value) -> Result<Vec<ClientEventType>, DatabaseError> {
    serde_json::from_value(value).map_err(|e| DatabaseError::FindMany(e.to_string()))
}

fn stored_subscription(model: webhook_subscription::Model) -> Result<StoredWebhookSubscription, DatabaseError> {
    Ok(StoredWebhookSubscription {
        id: model.id,
        account_id: model.account_id,
        wallet_id: model.wallet_id,
        url: model.url,
        event_types: parse_event_types(model.event_types)?,
        signing_secret: model.signing_secret,
        active: model.active,
        last_event_id: model.last_event_id,
        created_at: model.created_at.and_utc(),
        updated_at: model.updated_at.map(|value| value.and_utc()),
    })
}

fn client_event(model: client_event::Model) -> Result<ClientEvent, DatabaseError> {
    Ok(ClientEvent {
        id: model.id.to_string(),
        event_type: model
            .event_type
            .parse::<ClientEventType>()
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?,
        wallet_id: model.wallet_id,
        resource_id: model.resource_id,
        data: model.payload,
        created_at: model.created_at.and_utc(),
    })
}

fn delivery(model: webhook_delivery::Model) -> Result<WebhookDelivery, DatabaseError> {
    Ok(WebhookDelivery {
        id: model.id,
        subscription_id: model.subscription_id,
        event_id: model.client_event_id.to_string(),
        status: model
            .status
            .parse::<WebhookDeliveryStatus>()
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?,
        attempt_count: model.attempt_count as u32,
        response_status: model.response_status.map(|value| value as u16),
        last_error: model.last_error,
        delivered_at: model.delivered_at.map(|value| value.and_utc()),
        created_at: model.created_at.and_utc(),
        updated_at: model.updated_at.map(|value| value.and_utc()),
    })
}

#[async_trait]
impl WebhookRepository for SeaOrmWebhookRepository {
    async fn insert(&self, subscription: NewWebhookSubscription) -> Result<StoredWebhookSubscription, DatabaseError> {
        let event_types =
            serde_json::to_value(&subscription.event_types).map_err(|e| DatabaseError::Insert(e.to_string()))?;
        let model = webhook_subscription::ActiveModel {
            id: Set(subscription.id),
            account_id: Set(subscription.account_id),
            wallet_id: Set(subscription.wallet_id),
            url: Set(subscription.url),
            event_types: Set(event_types),
            signing_secret: Set(subscription.signing_secret),
            active: Set(true),
            last_event_id: Set(subscription.last_event_id),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        stored_subscription(model)
    }

    async fn find_many(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
    ) -> Result<Vec<StoredWebhookSubscription>, DatabaseError> {
        WebhookSubscriptionEntity::find()
            .filter(webhook_subscription::Column::AccountId.eq(account_id))
            .filter(webhook_subscription::Column::WalletId.eq(wallet_id))
            .order_by_asc(webhook_subscription::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?
            .into_iter()
            .map(stored_subscription)
            .collect()
    }

    async fn find_owned(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        id: Uuid,
    ) -> Result<Option<StoredWebhookSubscription>, DatabaseError> {
        WebhookSubscriptionEntity::find_by_id(id)
            .filter(webhook_subscription::Column::AccountId.eq(account_id))
            .filter(webhook_subscription::Column::WalletId.eq(wallet_id))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .map(stored_subscription)
            .transpose()
    }

    async fn update(
        &self,
        subscription: StoredWebhookSubscription,
    ) -> Result<StoredWebhookSubscription, DatabaseError> {
        let event_types =
            serde_json::to_value(&subscription.event_types).map_err(|e| DatabaseError::Update(e.to_string()))?;
        let model = webhook_subscription::ActiveModel {
            id: Unchanged(subscription.id),
            account_id: Unchanged(subscription.account_id),
            wallet_id: Unchanged(subscription.wallet_id),
            url: Set(subscription.url),
            event_types: Set(event_types),
            signing_secret: Set(subscription.signing_secret),
            active: Set(subscription.active),
            last_event_id: Set(subscription.last_event_id),
            updated_at: Set(Some(Utc::now().naive_utc())),
            ..Default::default()
        }
        .update(&self.db)
        .await
        .map_err(|e| DatabaseError::Update(e.to_string()))?;

        stored_subscription(model)
    }

    async fn cancel_pending(&self, subscription_id: Uuid, reason: String) -> Result<u64, DatabaseError> {
        let result = WebhookDeliveryEntity::update_many()
            .col_expr(webhook_delivery::Column::Status, Expr::value(EXHAUSTED))
            .col_expr(webhook_delivery::Column::LastError, Expr::value(Some(reason)))
            .col_expr(
                webhook_delivery::Column::LockedUntil,
                Expr::value(Option::<chrono::NaiveDateTime>::None),
            )
            .col_expr(
                webhook_delivery::Column::UpdatedAt,
                Expr::value(Some(Utc::now().naive_utc())),
            )
            .filter(webhook_delivery::Column::SubscriptionId.eq(subscription_id))
            .filter(webhook_delivery::Column::Status.eq(PENDING))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;
        Ok(result.rows_affected)
    }

    async fn delete_owned(&self, account_id: Uuid, wallet_id: Uuid, id: Uuid) -> Result<u64, DatabaseError> {
        let result = WebhookSubscriptionEntity::delete_many()
            .filter(webhook_subscription::Column::Id.eq(id))
            .filter(webhook_subscription::Column::AccountId.eq(account_id))
            .filter(webhook_subscription::Column::WalletId.eq(wallet_id))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;
        Ok(result.rows_affected)
    }

    async fn list_deliveries(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        subscription_id: Uuid,
        limit: u64,
    ) -> Result<Vec<WebhookDelivery>, DatabaseError> {
        let owned = WebhookSubscriptionEntity::find_by_id(subscription_id)
            .filter(webhook_subscription::Column::AccountId.eq(account_id))
            .filter(webhook_subscription::Column::WalletId.eq(wallet_id))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .is_some();
        if !owned {
            return Ok(Vec::new());
        }

        WebhookDeliveryEntity::find()
            .filter(webhook_delivery::Column::SubscriptionId.eq(subscription_id))
            .order_by_desc(webhook_delivery::Column::CreatedAt)
            .order_by_desc(webhook_delivery::Column::ClientEventId)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?
            .into_iter()
            .map(delivery)
            .collect()
    }

    async fn prepare_deliveries(&self, batch_size: u64) -> Result<u64, DatabaseError> {
        let transaction = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        let subscriptions = WebhookSubscriptionEntity::find()
            .order_by_asc(webhook_subscription::Column::CreatedAt)
            .all(&transaction)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;
        let mut prepared = 0;

        for subscription in subscriptions {
            let event_types = parse_event_types(subscription.event_types.clone())?;
            let events = ClientEventEntity::find()
                .filter(client_event::Column::WalletId.eq(subscription.wallet_id))
                .filter(client_event::Column::Id.gt(subscription.last_event_id))
                .order_by_asc(client_event::Column::Id)
                .limit(batch_size)
                .all(&transaction)
                .await
                .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

            for event in &events {
                let event_type = event
                    .event_type
                    .parse::<ClientEventType>()
                    .map_err(|e| DatabaseError::FindMany(e.to_string()))?;
                if subscription.active && event_types.contains(&event_type) {
                    WebhookDeliveryEntity::insert(webhook_delivery::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        subscription_id: Set(subscription.id),
                        client_event_id: Set(event.id),
                        status: Set(PENDING.to_string()),
                        attempt_count: Set(0),
                        next_attempt_at: Set(Utc::now().naive_utc()),
                        ..Default::default()
                    })
                    .on_conflict(
                        OnConflict::columns([
                            webhook_delivery::Column::SubscriptionId,
                            webhook_delivery::Column::ClientEventId,
                        ])
                        .do_nothing()
                        .to_owned(),
                    )
                    .exec_without_returning(&transaction)
                    .await
                    .map_err(|e| DatabaseError::Insert(e.to_string()))?;
                    prepared += 1;
                }
            }

            if let Some(last_event) = events.last() {
                WebhookSubscriptionEntity::update_many()
                    .col_expr(webhook_subscription::Column::LastEventId, Expr::value(last_event.id))
                    .filter(webhook_subscription::Column::Id.eq(subscription.id))
                    .exec(&transaction)
                    .await
                    .map_err(|e| DatabaseError::Update(e.to_string()))?;
            }
        }

        transaction
            .commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        Ok(prepared)
    }

    async fn claim_due(
        &self,
        now: DateTime<Utc>,
        locked_until: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<ClaimedWebhookDelivery>, DatabaseError> {
        let transaction = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        let lease_available = Condition::any()
            .add(webhook_delivery::Column::LockedUntil.is_null())
            .add(webhook_delivery::Column::LockedUntil.lt(now.naive_utc()));
        let candidates = WebhookDeliveryEntity::find()
            .filter(webhook_delivery::Column::Status.eq(PENDING))
            .filter(webhook_delivery::Column::NextAttemptAt.lte(now.naive_utc()))
            .filter(lease_available.clone())
            .order_by_asc(webhook_delivery::Column::NextAttemptAt)
            .limit(limit)
            .all(&transaction)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;
        let mut claimed = Vec::with_capacity(candidates.len());

        for candidate in candidates {
            let Some(subscription) = WebhookSubscriptionEntity::find_by_id(candidate.subscription_id)
                .one(&transaction)
                .await
                .map_err(|e| DatabaseError::FindOne(e.to_string()))?
                .filter(|subscription| subscription.active)
            else {
                continue;
            };

            let result = WebhookDeliveryEntity::update_many()
                .col_expr(
                    webhook_delivery::Column::LockedUntil,
                    Expr::value(Some(locked_until.naive_utc())),
                )
                .filter(webhook_delivery::Column::Id.eq(candidate.id))
                .filter(webhook_delivery::Column::Status.eq(PENDING))
                .filter(webhook_delivery::Column::NextAttemptAt.lte(now.naive_utc()))
                .filter(lease_available.clone())
                .exec(&transaction)
                .await
                .map_err(|e| DatabaseError::Update(e.to_string()))?;
            if result.rows_affected != 1 {
                continue;
            }

            let event = ClientEventEntity::find_by_id(candidate.client_event_id)
                .one(&transaction)
                .await
                .map_err(|e| DatabaseError::FindOne(e.to_string()))?
                .ok_or_else(|| DatabaseError::FindOne("Webhook event no longer exists.".to_string()))?;
            claimed.push(ClaimedWebhookDelivery {
                id: candidate.id,
                subscription_id: subscription.id,
                event: client_event(event)?,
                url: subscription.url,
                signing_secret: subscription.signing_secret,
                attempt_count: candidate.attempt_count as u32,
            });
        }

        transaction
            .commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        Ok(claimed)
    }

    async fn mark_delivered(&self, id: Uuid, response_status: u16) -> Result<(), DatabaseError> {
        WebhookDeliveryEntity::update_many()
            .col_expr(webhook_delivery::Column::Status, Expr::value(DELIVERED))
            .col_expr(
                webhook_delivery::Column::AttemptCount,
                Expr::col(webhook_delivery::Column::AttemptCount).add(1),
            )
            .col_expr(
                webhook_delivery::Column::ResponseStatus,
                Expr::value(Some(response_status as i32)),
            )
            .col_expr(webhook_delivery::Column::LastError, Expr::value(Option::<String>::None))
            .col_expr(
                webhook_delivery::Column::DeliveredAt,
                Expr::value(Some(Utc::now().naive_utc())),
            )
            .col_expr(
                webhook_delivery::Column::LockedUntil,
                Expr::value(Option::<chrono::NaiveDateTime>::None),
            )
            .col_expr(
                webhook_delivery::Column::UpdatedAt,
                Expr::value(Some(Utc::now().naive_utc())),
            )
            .filter(webhook_delivery::Column::Id.eq(id))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;
        Ok(())
    }

    async fn mark_failed(
        &self,
        id: Uuid,
        response_status: Option<u16>,
        error: String,
        next_attempt_at: DateTime<Utc>,
        exhausted: bool,
    ) -> Result<(), DatabaseError> {
        WebhookDeliveryEntity::update_many()
            .col_expr(
                webhook_delivery::Column::Status,
                Expr::value(if exhausted { EXHAUSTED } else { PENDING }),
            )
            .col_expr(
                webhook_delivery::Column::AttemptCount,
                Expr::col(webhook_delivery::Column::AttemptCount).add(1),
            )
            .col_expr(
                webhook_delivery::Column::ResponseStatus,
                Expr::value(response_status.map(|value| value as i32)),
            )
            .col_expr(webhook_delivery::Column::LastError, Expr::value(Some(error)))
            .col_expr(
                webhook_delivery::Column::NextAttemptAt,
                Expr::value(next_attempt_at.naive_utc()),
            )
            .col_expr(
                webhook_delivery::Column::LockedUntil,
                Expr::value(Option::<chrono::NaiveDateTime>::None),
            )
            .col_expr(
                webhook_delivery::Column::UpdatedAt,
                Expr::value(Some(Utc::now().naive_utc())),
            )
            .filter(webhook_delivery::Column::Id.eq(id))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;
        Ok(())
    }
}

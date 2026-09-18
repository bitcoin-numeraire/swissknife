use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sea_orm::{
    sea_query::{Expr, OnConflict},
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait, ExprTrait,
    QueryFilter, QueryOrder, QuerySelect, QueryTrait, Set, SqlErr, TransactionTrait,
};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::event::{
        ClaimedWebhookDelivery, ClientEventType, NewWebhookSubscription, StoredWebhookSubscription,
        UpdateWebhookSubscriptionRequest, WebhookDelivery, WebhookRepository, WebhookSubscriptionFilter,
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

use super::lock_client_event_log;
use crate::infra::database::sea_orm::sea_order;

const PENDING: &str = "Pending";
const DELIVERED: &str = "Delivered";
const EXHAUSTED: &str = "Exhausted";

fn insert_error(error: DbErr) -> DatabaseError {
    if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
        DatabaseError::Conflict(error.to_string())
    } else {
        DatabaseError::Insert(error.to_string())
    }
}

fn update_error(error: DbErr) -> DatabaseError {
    if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
        DatabaseError::Conflict(error.to_string())
    } else {
        DatabaseError::Update(error.to_string())
    }
}

#[derive(Clone)]
pub struct SeaOrmWebhookRepository {
    db: DatabaseConnection,
}

impl SeaOrmWebhookRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

async fn latest_wallet_event(tx: &DatabaseTransaction, wallet_id: Uuid) -> Result<i32, DatabaseError> {
    Ok(ClientEventEntity::find()
        .filter(client_event::Column::WalletId.eq(wallet_id))
        .order_by_desc(client_event::Column::Id)
        .one(tx)
        .await
        .map_err(|e| DatabaseError::FindOne(e.to_string()))?
        .map(|event| event.id)
        .unwrap_or_default())
}

#[async_trait]
impl WebhookRepository for SeaOrmWebhookRepository {
    async fn insert(&self, subscription: NewWebhookSubscription) -> Result<StoredWebhookSubscription, DatabaseError> {
        let tx = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        lock_client_event_log(&tx).await?;
        let last_event_id = latest_wallet_event(&tx, subscription.wallet_id).await?;
        let event_types =
            serde_json::to_value(&subscription.event_types).expect("should serialize successfully by assertion");
        let model = webhook_subscription::ActiveModel {
            id: Set(subscription.id),
            account_id: Set(subscription.account_id),
            wallet_id: Set(subscription.wallet_id),
            url: Set(subscription.url),
            event_types: Set(event_types),
            signing_secret: Set(subscription.signing_secret),
            active: Set(true),
            last_event_id: Set(last_event_id),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(insert_error)?;

        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        Ok(model.into())
    }

    async fn find(&self, id: Uuid) -> Result<Option<StoredWebhookSubscription>, DatabaseError> {
        Ok(WebhookSubscriptionEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .map(Into::into))
    }

    async fn find_many(
        &self,
        filter: WebhookSubscriptionFilter,
    ) -> Result<Vec<StoredWebhookSubscription>, DatabaseError> {
        Ok(WebhookSubscriptionEntity::find()
            .apply_if(filter.account_id, |q, id| {
                q.filter(webhook_subscription::Column::AccountId.eq(id))
            })
            .apply_if(filter.wallet_id, |q, id| {
                q.filter(webhook_subscription::Column::WalletId.eq(id))
            })
            .apply_if(filter.ids, |q, ids| {
                q.filter(webhook_subscription::Column::Id.is_in(ids))
            })
            .apply_if(filter.active, |q, active| {
                q.filter(webhook_subscription::Column::Active.eq(active))
            })
            .order_by(
                webhook_subscription::Column::CreatedAt,
                sea_order(&filter.order_direction),
            )
            .order_by(webhook_subscription::Column::Id, sea_order(&filter.order_direction))
            .offset(filter.offset)
            .limit(filter.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn find_owned(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        id: Uuid,
    ) -> Result<Option<StoredWebhookSubscription>, DatabaseError> {
        Ok(WebhookSubscriptionEntity::find_by_id(id)
            .filter(webhook_subscription::Column::AccountId.eq(account_id))
            .filter(webhook_subscription::Column::WalletId.eq(wallet_id))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .map(Into::into))
    }

    async fn update(
        &self,
        id: Uuid,
        request: UpdateWebhookSubscriptionRequest,
    ) -> Result<StoredWebhookSubscription, DatabaseError> {
        let tx = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        lock_client_event_log(&tx).await?;
        let current = WebhookSubscriptionEntity::find_by_id(id)
            .one(&tx)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .ok_or_else(|| DatabaseError::FindOne("Webhook subscription no longer exists.".to_string()))?;
        let transition = request.active.filter(|active| *active != current.active);
        let mut model: webhook_subscription::ActiveModel = current.clone().into();
        if let Some(url) = request.url {
            model.url = Set(url);
        }
        if let Some(events) = request.event_types {
            model.event_types = Set(serde_json::to_value(events).expect("should serialize successfully by assertion"));
        }
        if let Some(active) = transition {
            model.active = Set(active);
            model.last_event_id = Set(latest_wallet_event(&tx, current.wallet_id).await?);
            if !active {
                WebhookDeliveryEntity::update_many()
                    .col_expr(webhook_delivery::Column::Status, Expr::value(EXHAUSTED))
                    .col_expr(
                        webhook_delivery::Column::LastError,
                        Expr::value("Subscription disabled."),
                    )
                    .col_expr(
                        webhook_delivery::Column::LockedUntil,
                        Expr::value(Option::<NaiveDateTime>::None),
                    )
                    .col_expr(
                        webhook_delivery::Column::UpdatedAt,
                        Expr::value(Some(Utc::now().naive_utc())),
                    )
                    .filter(webhook_delivery::Column::SubscriptionId.eq(id))
                    .filter(webhook_delivery::Column::Status.eq(PENDING))
                    .exec(&tx)
                    .await
                    .map_err(|e| DatabaseError::Update(e.to_string()))?;
            }
        }
        model.updated_at = Set(Some(Utc::now().naive_utc()));
        let model = model.update(&tx).await.map_err(update_error)?;
        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        Ok(model.into())
    }

    async fn rotate_secret(&self, id: Uuid, signing_secret: String) -> Result<(), DatabaseError> {
        WebhookSubscriptionEntity::update_many()
            .col_expr(webhook_subscription::Column::SigningSecret, Expr::value(signing_secret))
            .col_expr(
                webhook_subscription::Column::UpdatedAt,
                Expr::value(Some(Utc::now().naive_utc())),
            )
            .filter(webhook_subscription::Column::Id.eq(id))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<u64, DatabaseError> {
        let result = WebhookSubscriptionEntity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;
        Ok(result.rows_affected)
    }

    async fn list_deliveries(&self, subscription_id: Uuid, limit: u64) -> Result<Vec<WebhookDelivery>, DatabaseError> {
        Ok(WebhookDeliveryEntity::find()
            .filter(webhook_delivery::Column::SubscriptionId.eq(subscription_id))
            .order_by_desc(webhook_delivery::Column::CreatedAt)
            .order_by_desc(webhook_delivery::Column::ClientEventId)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn prepare_deliveries(&self, batch_size: u64) -> Result<u64, DatabaseError> {
        let transaction = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        lock_client_event_log(&transaction).await?;
        let subscriptions: Vec<StoredWebhookSubscription> = WebhookSubscriptionEntity::find()
            .order_by_asc(webhook_subscription::Column::CreatedAt)
            .all(&transaction)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?
            .into_iter()
            .map(Into::into)
            .collect();
        let mut prepared = 0;

        for subscription in subscriptions {
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
                    .expect("should parse successfully by assertion");
                if subscription.active && subscription.event_types.contains(&event_type) {
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
                event: event.into(),
                url: subscription.url,
                signing_secret: subscription.signing_secret,
                attempt_count: candidate
                    .attempt_count
                    .try_into()
                    .expect("should parse successfully by assertion"),
                lease_expires_at: locked_until,
            });
        }

        transaction
            .commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        Ok(claimed)
    }

    async fn mark_delivered(
        &self,
        id: Uuid,
        lease_expires_at: DateTime<Utc>,
        response_status: u16,
    ) -> Result<(), DatabaseError> {
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
                Expr::value(Option::<NaiveDateTime>::None),
            )
            .col_expr(
                webhook_delivery::Column::UpdatedAt,
                Expr::value(Some(Utc::now().naive_utc())),
            )
            .filter(webhook_delivery::Column::Id.eq(id))
            .filter(webhook_delivery::Column::Status.eq(PENDING))
            .filter(webhook_delivery::Column::LockedUntil.eq(lease_expires_at.naive_utc()))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;
        Ok(())
    }

    async fn mark_failed(
        &self,
        id: Uuid,
        lease_expires_at: DateTime<Utc>,
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
                Expr::value(Option::<NaiveDateTime>::None),
            )
            .col_expr(
                webhook_delivery::Column::UpdatedAt,
                Expr::value(Some(Utc::now().naive_utc())),
            )
            .filter(webhook_delivery::Column::Id.eq(id))
            .filter(webhook_delivery::Column::Status.eq(PENDING))
            .filter(webhook_delivery::Column::LockedUntil.eq(lease_expires_at.naive_utc()))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;
        Ok(())
    }
}

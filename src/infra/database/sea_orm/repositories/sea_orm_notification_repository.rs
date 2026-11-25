use async_trait::async_trait;
use sea_orm::{
    sea_query::Expr, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use swissknife_types::{AccountNotification, NotificationAction};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::notification::NotificationRepository,
    infra::database::sea_orm::models::{
        account_notification::{self, Column},
        client_event,
        prelude::AccountNotification as Entity,
    },
};

pub struct SeaOrmNotificationRepository {
    db: DatabaseConnection,
}

impl SeaOrmNotificationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn notification(
    state: account_notification::Model,
    event: client_event::Model,
) -> Result<AccountNotification, DatabaseError> {
    let data = &event.payload;
    let amount_msat = data
        .get("amount_received_msat")
        .and_then(|v| v.as_u64())
        .or_else(|| data.get("amount_msat").and_then(|v| v.as_u64()))
        .map(|amount| amount.to_string());
    Ok(AccountNotification {
        source_event_id: state.source_event_id,
        event_type: event
            .event_type
            .parse()
            .map_err(|_| DatabaseError::FindOne("Unrecognized notification event".into()))?,
        wallet_id: event.wallet_id,
        resource_id: event.resource_id,
        created_at: event.created_at.and_utc(),
        ledger: data
            .get("ledger")
            .and_then(|value| serde_json::from_value(value.clone()).ok()),
        amount_msat,
        description: data.get("description").and_then(|v| v.as_str()).map(str::to_owned),
        is_read: state.is_read,
        archived: state.archived,
    })
}

#[async_trait]
impl NotificationRepository for SeaOrmNotificationRepository {
    async fn list(
        &self,
        account_id: Uuid,
        before: Option<i32>,
        limit: u64,
        archived: bool,
    ) -> Result<Vec<AccountNotification>, DatabaseError> {
        let mut query = Entity::find()
            .filter(Column::AccountId.eq(account_id))
            .filter(Column::Archived.eq(archived));
        if let Some(before) = before {
            query = query.filter(Column::SourceEventId.lt(before));
        }
        let rows = query
            .find_also_related(client_event::Entity)
            .order_by_desc(Column::SourceEventId)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;
        rows.into_iter()
            .map(|(state, event)| {
                notification(
                    state,
                    event.ok_or_else(|| DatabaseError::FindRelated("Notification source event".into()))?,
                )
            })
            .collect()
    }

    async fn unread_count(&self, account_id: Uuid) -> Result<u64, DatabaseError> {
        Entity::find()
            .filter(Column::AccountId.eq(account_id))
            .filter(Column::Archived.eq(false))
            .filter(Column::IsRead.eq(false))
            .count(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))
    }

    async fn update(&self, account_id: Uuid, id: i32, action: NotificationAction) -> Result<bool, DatabaseError> {
        let mut update = Entity::update_many()
            .col_expr(Column::IsRead, Expr::value(true))
            .filter(Column::AccountId.eq(account_id))
            .filter(Column::SourceEventId.eq(id));
        if matches!(action, NotificationAction::Archive) {
            update = update.col_expr(Column::Archived, Expr::value(true));
        }
        Ok(update
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?
            .rows_affected
            != 0)
    }

    async fn read_through(&self, account_id: Uuid, through: i32) -> Result<(), DatabaseError> {
        Entity::update_many()
            .col_expr(Column::IsRead, Expr::value(true))
            .filter(Column::AccountId.eq(account_id))
            .filter(Column::SourceEventId.lte(through))
            .filter(Column::IsRead.eq(false))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;
        Ok(())
    }
}

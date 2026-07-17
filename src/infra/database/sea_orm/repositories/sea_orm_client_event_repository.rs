use async_trait::async_trait;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use super::SeaOrmConnection;
use crate::{
    application::errors::DatabaseError,
    domains::event::{ClientEvent, ClientEventRepository, ClientEventType, NewClientEvent},
    infra::database::sea_orm::models::{
        client_event::{ActiveModel, Column, Model},
        prelude::ClientEvent as ClientEventEntity,
    },
};

#[derive(Clone)]
pub struct SeaOrmClientEventRepository<C = DatabaseConnection> {
    db: C,
}

impl<C> SeaOrmClientEventRepository<C> {
    pub fn new(db: C) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<C> ClientEventRepository for SeaOrmClientEventRepository<C>
where
    C: SeaOrmConnection,
{
    async fn append(&self, event: NewClientEvent) -> Result<(), DatabaseError> {
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
        .exec_without_returning(self.db.connection())
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(())
    }

    async fn latest_id(&self, wallet_id: Uuid) -> Result<Option<i32>, DatabaseError> {
        Ok(ClientEventEntity::find()
            .filter(Column::WalletId.eq(wallet_id))
            .order_by_desc(Column::Id)
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .map(|model| model.id))
    }

    async fn find_after(&self, wallet_id: Uuid, after_id: i32, limit: u64) -> Result<Vec<ClientEvent>, DatabaseError> {
        ClientEventEntity::find()
            .filter(Column::WalletId.eq(wallet_id))
            .filter(Column::Id.gt(after_id))
            .order_by_asc(Column::Id)
            .limit(limit)
            .all(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
}

impl TryFrom<Model> for ClientEvent {
    type Error = DatabaseError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        let event_type = model
            .event_type
            .parse::<ClientEventType>()
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(Self {
            id: model.id.to_string(),
            event_type,
            wallet_id: model.wallet_id,
            resource_id: model.resource_id,
            data: model.payload,
            created_at: model.created_at.and_utc(),
        })
    }
}

use async_trait::async_trait;
use nostr_sdk::PublicKey;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Set, Unchanged,
};
use uuid::Uuid;

use crate::application::errors::DatabaseError;
use crate::domains::ln_address::{LnAddress, LnAddressFilter, LnAddressRepository};
use crate::infra::database::sea_orm::models::ln_address::{ActiveModel, Column, Entity};

#[derive(Clone)]
pub struct SeaOrmLnAddressRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmLnAddressRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LnAddressRepository for SeaOrmLnAddressRepository {
    async fn find(&self, id: Uuid) -> Result<Option<LnAddress>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_by_wallet_id(&self, wallet_id: Uuid) -> Result<Option<LnAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::WalletId.eq(wallet_id))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<LnAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_many(&self, filter: LnAddressFilter) -> Result<Vec<LnAddress>, DatabaseError> {
        let models = Entity::find()
            .apply_if(filter.wallet_id, |q, id| q.filter(Column::WalletId.eq(id)))
            .apply_if(filter.username, |q, username| {
                q.filter(Column::Username.eq(username))
            })
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.active, |q, active| {
                q.filter(Column::Active.eq(active))
            })
            .order_by(Column::CreatedAt, filter.order_direction.into())
            .offset(filter.offset)
            .limit(filter.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn insert(
        &self,
        wallet_id: Uuid,
        username: &str,
        allows_nostr: bool,
        nostr_pubkey: Option<PublicKey>,
    ) -> Result<LnAddress, DatabaseError> {
        let model = ActiveModel {
            wallet_id: Set(wallet_id),
            username: Set(username.to_owned()),
            allows_nostr: Set(allows_nostr),
            nostr_pubkey: Set(nostr_pubkey.map(|k| k.to_hex())),
            active: Set(true),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn update(&self, ln_address: LnAddress) -> Result<LnAddress, DatabaseError> {
        let model = ActiveModel {
            id: Unchanged(ln_address.id),
            wallet_id: Unchanged(ln_address.wallet_id),
            username: Set(ln_address.username),
            allows_nostr: Set(ln_address.allows_nostr),
            nostr_pubkey: Set(ln_address.nostr_pubkey.map(|k| k.to_hex())),
            active: Set(ln_address.active),
            ..Default::default()
        };

        let model = model
            .update(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(model.into())
    }

    async fn delete_many(&self, filter: LnAddressFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.wallet_id, |q, id| q.filter(Column::WalletId.eq(id)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.username, |q, username| {
                q.filter(Column::Username.eq(username))
            })
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

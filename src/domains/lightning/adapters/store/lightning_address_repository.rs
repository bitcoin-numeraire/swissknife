use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait,
    FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Statement,
};

use crate::domains::lightning::adapters::models::lightning_address::{ActiveModel, Column, Entity};
use crate::domains::lightning::adapters::models::user_balance::UserBalanceModel;
use crate::domains::lightning::adapters::repository::LightningAddressRepository;
use crate::{
    application::errors::DatabaseError,
    domains::lightning::entities::{LightningAddress, UserBalance},
};

use super::LightningStore;

#[async_trait]
impl LightningAddressRepository for LightningStore {
    async fn find_address_by_user_id(
        &self,
        user_id: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_address_by_username(
        &self,
        username: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_all_addresses(
        &self,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, DatabaseError> {
        let models = Entity::find()
            .order_by_asc(Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindAll(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn find_all_addresses_by_user_id(
        &self,
        user_id: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, DatabaseError> {
        let models = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by_asc(Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindAll(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn insert_address(
        &self,
        user_id: &str,
        username: &str,
    ) -> Result<LightningAddress, DatabaseError> {
        let model = ActiveModel {
            user_id: Set(user_id.to_owned()),
            username: Set(username.to_owned()),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn get_balance_by_username(
        &self,
        txn: Option<&DatabaseTransaction>,
        username: &str,
    ) -> Result<UserBalance, DatabaseError> {
        let query = UserBalanceModel::find_by_statement(Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            WITH sent AS (
                SELECT
                    COALESCE(SUM(CASE WHEN status = 'SETTLED' THEN amount_msat ELSE 0 END) -
                    COALESCE(SUM(CASE WHEN status = 'PENDING' THEN amount_msat ELSE 0 END), 0), 0)::BIGINT AS sent_msat,
                    COALESCE(SUM(CASE WHEN status = 'SETTLED' THEN COALESCE(fee_msat, 0) ELSE 0 END), 0)::BIGINT AS fees_paid_msat
                FROM lightning_payment
                WHERE lightning_address = $1
            ),
            received AS (
                SELECT
                    COALESCE(SUM(CASE WHEN status = 'SETTLED' THEN amount_msat ELSE 0 END), 0)::BIGINT AS received_msat
                FROM lightning_invoice
                WHERE lightning_address = $1
            )
            SELECT
                COALESCE(received.received_msat, 0) AS received_msat,
                COALESCE(sent.sent_msat, 0) AS sent_msat,
                COALESCE(sent.fees_paid_msat, 0) AS fees_paid_msat,
                COALESCE((received.received_msat - (sent.sent_msat + sent.fees_paid_msat)), 0) AS available_msat
            FROM received, sent;
            "#,
            [username.into()],
        ));

        let result = match txn {
            Some(txn) => query.one(txn).await,
            None => query.one(&self.db).await,
        };

        let result = result.map_err(|e| DatabaseError::FindByStatement(e.to_string()))?;

        match result {
            Some(model) => Ok(model.into()),
            None => Ok(UserBalance::default()),
        }
    }
}

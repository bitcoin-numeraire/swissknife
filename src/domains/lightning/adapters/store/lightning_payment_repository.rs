use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Unchanged,
};
use uuid::Uuid;

use crate::domains::lightning::adapters::models::lightning_payment::{ActiveModel, Column, Entity};
use crate::domains::lightning::adapters::repository::LightningPaymentRepository;
use crate::{application::errors::DatabaseError, domains::lightning::entities::LightningPayment};

use super::LightningStore;

#[async_trait]
impl LightningPaymentRepository for LightningStore {
    async fn find_payment(&self, id: Uuid) -> Result<Option<LightningPayment>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_all_payments(
        &self,
        user: Option<String>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningPayment>, DatabaseError> {
        let models = Entity::find()
            .apply_if(user, |q, v| q.filter(Column::UserId.eq(v)))
            .order_by_desc(Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindAll(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn insert_payment(
        &self,
        txn: Option<&DatabaseTransaction>,
        payment: LightningPayment,
    ) -> Result<LightningPayment, DatabaseError> {
        let model = ActiveModel {
            user_id: Set(payment.user_id),
            lightning_address: Set(payment.lightning_address),
            amount_msat: Set(payment.amount_msat as i64),
            status: Set(payment.status.to_string()),
            payment_hash: Set(payment.payment_hash),
            description: Set(payment.description),
            ..Default::default()
        };

        let result = match txn {
            Some(txn) => model.insert(txn).await,
            None => model.insert(&self.db).await,
        };

        let model = result.map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn update_payment(
        &self,
        payment: LightningPayment,
    ) -> Result<LightningPayment, DatabaseError> {
        let mut model = ActiveModel {
            id: Set(payment.id),
            status: Set(payment.status.to_string()),
            fee_msat: Set(payment.fee_msat.map(|v| v as i64)),
            payment_time: Set(payment.payment_time),
            payment_hash: Set(payment.payment_hash),
            error: Set(payment.error),
            amount_msat: Set(payment.amount_msat as i64),
            metadata: Set(payment.metadata),
            ..Default::default()
        };

        // TODO: Remove when event contains success_action or when we retrieve payments by querying the API after queue implementation
        match payment.success_action {
            Some(_) => {
                model.success_action = Set(payment.success_action);
            }
            None => {
                model.success_action = Unchanged(payment.success_action);
            }
        }

        let model = model
            .update(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(model.into())
    }
}

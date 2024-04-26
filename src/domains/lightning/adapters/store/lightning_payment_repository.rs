use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Unchanged,
};
use uuid::Uuid;

use crate::domains::lightning::adapters::models::lightning_payment::{ActiveModel, Column, Entity};
use crate::domains::lightning::adapters::repository::LightningPaymentRepository;
use crate::domains::lightning::entities::LightningPaymentStatus;
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

    // TODO: Temporary before fix by Breez
    async fn find_payment_by_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningPayment>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::PaymentHash.eq(payment_hash))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn insert_payment(
        &self,
        txn: Option<&DatabaseTransaction>,
        lightning_address: Option<String>,
        status: LightningPaymentStatus,
        amount_msat: u64,
        payment_hash: Option<String>,
    ) -> Result<LightningPayment, DatabaseError> {
        let model = ActiveModel {
            lightning_address: Set(lightning_address),
            amount_msat: Set(amount_msat as i64),
            status: Set(status.to_string()),
            payment_hash: Set(payment_hash),
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
            payment_time: Set(payment.payment_time.map(|v| v as i64)),
            payment_hash: Set(payment.payment_hash),
            error: Set(payment.error),
            amount_msat: Set(payment.amount_msat as i64),
            description: Set(payment.description),
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

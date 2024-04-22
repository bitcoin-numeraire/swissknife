use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter};

use crate::domains::lightning::adapters::models::lightning_payment::{ActiveModel, Column, Entity};
use crate::domains::lightning::adapters::repository::LightningPaymentRepository;
use crate::{application::errors::DatabaseError, domains::lightning::entities::LightningPayment};

use super::LightningStore;

#[async_trait]
impl LightningPaymentRepository for LightningStore {
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
        status: String,
        amount_msat: u64,
    ) -> Result<LightningPayment, DatabaseError> {
        let model = ActiveModel {
            lightning_address: Set(lightning_address),
            amount_msat: Set(amount_msat as i64),
            status: Set(status),
            payment_hash: Set("test".to_string()),
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
        let model = ActiveModel {
            id: Set(payment.id),
            status: Set(payment.status),
            fee_msat: Set(payment.fee_msat.map(|v| v as i64)),
            payment_time: Set(payment.payment_time.map(|v| v as i64)),
            payment_hash: Set(payment.payment_hash),
            error: Set(payment.error),
            amount_msat: Set(payment.amount_msat as i64),
            description: Set(payment.description),
            metadata: Set(payment.metadata),
            ..Default::default()
        };

        let model = model
            .update(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(model.into())
    }
}

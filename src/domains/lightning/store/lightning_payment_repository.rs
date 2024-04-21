use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    application::errors::DatabaseError,
    domains::lightning::{entities::LightningPayment, store::LightningPaymentRepository},
};

use super::models::lightning_payment::{ActiveModel, Column, Entity};

#[derive(Clone)]
pub struct SqlLightningPaymentRepository {
    executor: DatabaseConnection,
}

impl SqlLightningPaymentRepository {
    pub fn new(executor: DatabaseConnection) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl LightningPaymentRepository for SqlLightningPaymentRepository {
    async fn find_by_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningPayment>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::PaymentHash.eq(payment_hash))
            .one(&self.executor)
            .await
            .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn insert(&self, payment: LightningPayment) -> Result<LightningPayment, DatabaseError> {
        let model = ActiveModel {
            lightning_address: Set(payment.lightning_address),
            payment_hash: Set(payment.payment_hash),
            error: Set(payment.error),
            amount_msat: Set(payment.amount_msat as i64),
            fee_msat: Set(payment.fee_msat.map(|v| v as i64)),
            payment_time: Set(payment.payment_time.map(|v| v as i64)),
            status: Set(payment.status),
            description: Set(payment.description),
            metadata: Set(payment.metadata),
            ..Default::default()
        };

        let model = model
            .insert(&self.executor)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn update(&self, payment: LightningPayment) -> Result<LightningPayment, DatabaseError> {
        let model = ActiveModel {
            id: Set(payment.id),
            status: Set(payment.status),
            fee_msat: Set(payment.fee_msat.map(|v| v as i64)),
            payment_time: Set(payment.payment_time.map(|v| v as i64)),
            ..Default::default()
        };

        let model = model
            .update(&self.executor)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(model.into())
    }
}

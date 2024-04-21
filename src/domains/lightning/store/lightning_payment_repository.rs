use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use crate::{
    application::errors::DatabaseError,
    domains::lightning::{entities::LightningPayment, store::LightningPaymentRepository},
};

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
        let result = sqlx::query_as!(
            LightningPayment,
            r#"
               SELECT * FROM "lightning_payments" WHERE payment_hash = $1
           "#,
            payment_hash
        )
        .fetch_optional(&self.executor)
        .await
        .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(result)
    }

    async fn insert(
        &self,
        executor: &DatabaseConnection,
        payment: LightningPayment,
    ) -> Result<LightningPayment, DatabaseError> {
        let query = sqlx::query_as!(
            LightningPayment,
            r#"
                INSERT INTO lightning_payments (
                    lightning_address,
                    payment_hash,
                    error,
                    amount_msat,
                    fee_msat,
                    payment_time,
                    status,
                    description,
                    metadata
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9
                ) RETURNING *;
            "#,
            payment.lightning_address,
            payment.payment_hash,
            payment.error,
            payment.amount_msat,
            payment.fee_msat,
            payment.payment_time,
            payment.status,
            payment.description,
            payment.metadata,
        );

        let result = if let Some(tx) = executor {
            query.fetch_one(&mut **tx).await
        } else {
            query.fetch_one(&self.executor).await
        };

        let lightning_payment = result.map_err(|e| DatabaseError::Save(e.to_string()))?;

        Ok(lightning_payment)
    }

    async fn update(&self, payment: LightningPayment) -> Result<LightningPayment, DatabaseError> {
        let lightning_payment = sqlx::query_as!(
            LightningPayment,
            r#"
                UPDATE lightning_payments
                SET
                    status = $1,
                    fee_msat = $2,
                    payment_time = $3
                WHERE payment_hash = $4
                RETURNING *;
            "#,
            payment.status,
            payment.fee_msat,
            payment.payment_time,
            payment.payment_hash
        )
        .fetch_one(&self.executor)
        .await
        .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(lightning_payment)
    }
}

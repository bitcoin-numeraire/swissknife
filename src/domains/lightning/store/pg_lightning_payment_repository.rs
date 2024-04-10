use async_trait::async_trait;

use crate::{
    adapters::database::DatabaseClient,
    application::errors::DatabaseError,
    domains::lightning::{entities::LightningPayment, store::LightningPaymentRepository},
};

#[derive(Clone)]
pub struct PgLightningPaymentRepository<D: DatabaseClient> {
    db_client: D,
}

impl<D: DatabaseClient> PgLightningPaymentRepository<D> {
    pub fn new(db_client: D) -> Self {
        Self { db_client }
    }
}

#[async_trait]
impl<D: DatabaseClient> LightningPaymentRepository for PgLightningPaymentRepository<D> {
    async fn get_by_hash(
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
        .fetch_optional(&self.db_client.pool())
        .await
        .map_err(|e| DatabaseError::Get(e.to_string()))?;

        Ok(result)
    }

    async fn insert(&self, payment: LightningPayment) -> Result<LightningPayment, DatabaseError> {
        let lightning_payment = sqlx::query_as!(
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
        )
        .fetch_one(&self.db_client.pool())
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;

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
        .fetch_one(&self.db_client.pool())
        .await
        .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(lightning_payment)
    }
}

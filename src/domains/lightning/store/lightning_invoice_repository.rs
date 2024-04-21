use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use crate::{
    application::errors::DatabaseError,
    domains::lightning::{entities::LightningInvoice, store::LightningInvoiceRepository},
};

#[derive(Clone)]
pub struct SqlLightningInvoiceRepository {
    executor: DatabaseConnection,
}

impl SqlLightningInvoiceRepository {
    pub fn new(executor: DatabaseConnection) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl LightningInvoiceRepository for SqlLightningInvoiceRepository {
    async fn find_by_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningInvoice>, DatabaseError> {
        let result = sqlx::query_as!(
            LightningInvoice,
            r#"
               SELECT * FROM "lightning_invoices" WHERE payment_hash = $1
           "#,
            payment_hash
        )
        .fetch_optional(&self.executor)
        .await
        .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(result)
    }

    async fn insert(&self, invoice: LightningInvoice) -> Result<LightningInvoice, DatabaseError> {
        let lightning_invoice = sqlx::query_as!(
            LightningInvoice,
            r#"
                INSERT INTO lightning_invoices (
                    lightning_address,
                    bolt11,
                    network,
                    payee_pubkey,
                    payment_hash,
                    description,
                    description_hash,
                    amount_msat,
                    payment_secret,
                    timestamp,
                    expiry,
                    min_final_cltv_expiry_delta,
                    status,
                    fee_msat,
                    payment_time
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
                ) RETURNING *;
            "#,
            invoice.lightning_address,
            invoice.bolt11,
            invoice.network,
            invoice.payee_pubkey,
            invoice.payment_hash,
            invoice.description,
            invoice.description_hash,
            invoice.amount_msat,
            invoice.payment_secret,
            invoice.timestamp,
            invoice.expiry,
            invoice.min_final_cltv_expiry_delta,
            invoice.status,
            invoice.fee_msat,
            invoice.payment_time
        )
        .fetch_one(&self.executor)
        .await
        .map_err(|e| DatabaseError::Save(e.to_string()))?;

        Ok(lightning_invoice)
    }

    async fn update(&self, invoice: LightningInvoice) -> Result<LightningInvoice, DatabaseError> {
        let lightning_invoice = sqlx::query_as!(
            LightningInvoice,
            r#"
                UPDATE lightning_invoices
                SET
                    status = $1,
                    fee_msat = $2,
                    payment_time = $3,
                    updated_at = NOW()
                WHERE payment_hash = $4
                RETURNING *;
            "#,
            invoice.status,
            invoice.fee_msat,
            invoice.payment_time,
            invoice.payment_hash
        )
        .fetch_one(&self.executor)
        .await
        .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(lightning_invoice)
    }
}

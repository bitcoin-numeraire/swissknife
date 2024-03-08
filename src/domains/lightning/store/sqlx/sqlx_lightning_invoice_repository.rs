use async_trait::async_trait;

use crate::{
    adapters::database::DatabaseClient,
    application::errors::DatabaseError,
    domains::lightning::{entities::LightningInvoice, store::LightningInvoiceRepository},
};

pub struct SqlxLightningInvoiceRepository<D: DatabaseClient> {
    db_client: D,
}

impl<D: DatabaseClient> SqlxLightningInvoiceRepository<D> {
    pub fn new(db_client: D) -> Self {
        Self { db_client }
    }
}

#[async_trait]
impl<D: DatabaseClient> LightningInvoiceRepository for SqlxLightningInvoiceRepository<D> {
    async fn insert(&self, invoice: LightningInvoice) -> Result<LightningInvoice, DatabaseError> {
        let lightning_invoice = sqlx::query_as!(
            LightningInvoice,
            // language=PostgreSQL
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
                    min_final_cltv_expiry_delta
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
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
            invoice.min_final_cltv_expiry_delta
        )
        .fetch_one(&self.db_client.pool())
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(lightning_invoice)
    }
}

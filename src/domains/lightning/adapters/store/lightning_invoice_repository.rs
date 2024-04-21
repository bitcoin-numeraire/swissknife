use crate::{
    application::errors::DatabaseError,
    domains::lightning::{
        adapters::{
            models::lightning_invoice::{ActiveModel, Column, Entity},
            repository::LightningInvoiceRepository,
        },
        entities::LightningInvoice,
    },
};
use async_trait::async_trait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

use super::LightningStore;

#[async_trait]
impl LightningInvoiceRepository for LightningStore {
    async fn find_invoice_by_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningInvoice>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::PaymentHash.eq(payment_hash))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn insert_invoice(
        &self,
        invoice: LightningInvoice,
    ) -> Result<LightningInvoice, DatabaseError> {
        let model = ActiveModel {
            lightning_address: Set(invoice.lightning_address),
            bolt11: Set(invoice.bolt11),
            network: Set(invoice.network),
            payee_pubkey: Set(invoice.payee_pubkey),
            payment_hash: Set(invoice.payment_hash),
            description: Set(invoice.description),
            description_hash: Set(invoice.description_hash),
            amount_msat: Set(invoice.amount_msat.map(|v| v as i64)),
            payment_secret: Set(invoice.payment_secret),
            timestamp: Set(invoice.timestamp as i64),
            expiry: Set(invoice.expiry as i64),
            min_final_cltv_expiry_delta: Set(invoice.min_final_cltv_expiry_delta as i64),
            status: Set(invoice.status),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn update_invoice(
        &self,
        invoice: LightningInvoice,
    ) -> Result<LightningInvoice, DatabaseError> {
        let model = ActiveModel {
            id: Set(invoice.id),
            status: Set(invoice.status),
            fee_msat: Set(invoice.fee_msat.map(|v| v as i64)),
            payment_time: Set(invoice.payment_time.map(|v| v as i64)),
            ..Default::default()
        };

        let model = model
            .update(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(model.into())
    }
}

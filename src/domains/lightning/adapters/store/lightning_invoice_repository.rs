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
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::LightningStore;

#[async_trait]
impl LightningInvoiceRepository for LightningStore {
    async fn find_invoice(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningInvoice>, DatabaseError> {
        let model = Entity::find_by_id(payment_hash)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::Find(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_all_invoices(
        &self,
        user: Option<String>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningInvoice>, DatabaseError> {
        let filter = match user {
            Some(user_id) => Entity::find().filter(Column::UserId.eq(user_id)),
            None => Entity::find(),
        };

        let models = filter
            .order_by_asc(Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindAll(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn insert_invoice(
        &self,
        invoice: LightningInvoice,
    ) -> Result<LightningInvoice, DatabaseError> {
        let model = ActiveModel {
            user_id: Set(invoice.user_id),
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
            details: Set(invoice.details),
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
            payment_hash: Set(invoice.payment_hash),
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

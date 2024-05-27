use crate::{
    application::errors::DatabaseError,
    domains::lightning::{
        adapters::{
            models::lightning_invoice::{ActiveModel, Column, Entity},
            repository::LightningInvoiceRepository,
        },
        entities::{LightningInvoice, LightningInvoiceFilter, LightningInvoiceStatus},
    },
};
use async_trait::async_trait;
use sea_orm::{sea_query::Expr, ActiveValue::Set, Condition, QueryTrait};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use super::LightningStore;

#[async_trait]
impl LightningInvoiceRepository for LightningStore {
    async fn find_invoice(&self, id: Uuid) -> Result<Option<LightningInvoice>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_invoice_by_payment_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<LightningInvoice>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::PaymentHash.eq(payment_hash))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_invoices(
        &self,
        filter: LightningInvoiceFilter,
    ) -> Result<Vec<LightningInvoice>, DatabaseError> {
        let models = Entity::find()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.id, |q, id| q.filter(Column::Id.eq(id)))
            .apply_if(filter.status, |q, s| match s {
                LightningInvoiceStatus::PENDING => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).gt(Expr::current_timestamp()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
                LightningInvoiceStatus::SETTLED => {
                    q.filter(Expr::col(Column::PaymentTime).is_not_null())
                }
                LightningInvoiceStatus::EXPIRED => {
                    q.filter(Expr::col(Column::ExpiresAt).lte(Expr::current_timestamp()))
                }
            })
            .order_by_desc(Column::CreatedAt)
            .offset(filter.offset)
            .limit(filter.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

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
            timestamp: Set(invoice.timestamp),
            expiry: Set(invoice.expiry.as_secs() as i64),
            min_final_cltv_expiry_delta: Set(invoice.min_final_cltv_expiry_delta as i64),
            label: Set(invoice.label),
            expires_at: Set(invoice.expires_at),
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
            payment_hash: Set(invoice.payment_hash),
            fee_msat: Set(invoice.fee_msat.map(|v| v as i64)),
            payment_time: Set(invoice.payment_time),
            description: Set(invoice.description),
            ..Default::default()
        };

        let model = model
            .update(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(model.into())
    }

    async fn delete_invoices(&self, filter: LightningInvoiceFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.id, |q, id| q.filter(Column::Id.eq(id)))
            .apply_if(filter.status, |q, status| match status {
                LightningInvoiceStatus::PENDING => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).gt(Expr::current_timestamp()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
                LightningInvoiceStatus::SETTLED => {
                    q.filter(Expr::col(Column::PaymentTime).is_not_null())
                }
                LightningInvoiceStatus::EXPIRED => {
                    q.filter(Expr::col(Column::ExpiresAt).lte(Expr::current_timestamp()))
                }
            })
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

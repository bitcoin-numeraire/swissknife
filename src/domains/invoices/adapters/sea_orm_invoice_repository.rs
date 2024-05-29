use crate::{
    application::errors::DatabaseError,
    domains::invoices::entities::{Invoice, InvoiceFilter, InvoiceStatus, InvoiceType},
};
use async_trait::async_trait;
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition,
    DatabaseConnection, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    QueryTrait,
};
use uuid::Uuid;

use super::{
    invoice_model::{ActiveModel, Column, Entity},
    InvoiceRepository,
};

#[derive(Clone)]
pub struct SeaOrmInvoiceRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmInvoiceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl InvoiceRepository for SeaOrmInvoiceRepository {
    async fn find(&self, id: Uuid) -> Result<Option<Invoice>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_by_payment_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<Invoice>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::PaymentHash.eq(payment_hash))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_many(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, DatabaseError> {
        let models = Entity::find()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.id, |q, id| q.filter(Column::Id.eq(id)))
            .apply_if(filter.status, |q, s| match s {
                InvoiceStatus::PENDING => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).gt(Expr::current_timestamp()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
                InvoiceStatus::SETTLED => q.filter(Expr::col(Column::PaymentTime).is_not_null()),
                InvoiceStatus::EXPIRED => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).lte(Expr::current_timestamp()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
            })
            .order_by_desc(Column::CreatedAt)
            .offset(filter.pagination.offset)
            .limit(filter.pagination.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn insert(
        &self,
        txn: Option<&DatabaseTransaction>,
        invoice: Invoice,
    ) -> Result<Invoice, DatabaseError> {
        let mut model = ActiveModel {
            user_id: Set(invoice.user_id),
            invoice_type: Set(invoice.invoice_type.to_string()),
            lightning_address: Set(invoice.lightning_address),
            network: Set(invoice.network),
            description: Set(invoice.description),
            amount_msat: Set(invoice.amount_msat.map(|v| v as i64)),
            timestamp: Set(invoice.timestamp),
            label: Set(invoice.label),
            ..Default::default()
        };

        if invoice.invoice_type == InvoiceType::LIGHTNING {
            let lightning = invoice.lightning.unwrap();
            model.bolt11 = Set(lightning.bolt11.into());
            model.payee_pubkey = Set(lightning.payee_pubkey.into());
            model.payment_hash = Set(lightning.payment_hash.into());
            model.description_hash = Set(lightning.description_hash);
            model.payment_secret = Set(lightning.payment_secret.into());
            model.min_final_cltv_expiry_delta =
                Set((lightning.min_final_cltv_expiry_delta as i64).into());
            model.expiry = Set((lightning.expiry.as_secs() as i64).into());
            model.expires_at = Set(lightning.expires_at.into());
        }

        let result = match txn {
            Some(txn) => model.insert(txn).await,
            None => model.insert(&self.db).await,
        };

        let model = result.map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn update(
        &self,
        txn: Option<&DatabaseTransaction>,
        invoice: Invoice,
    ) -> Result<Invoice, DatabaseError> {
        let model = ActiveModel {
            id: Set(invoice.id),
            fee_msat: Set(invoice.fee_msat.map(|v| v as i64)),
            payment_time: Set(invoice.payment_time),
            description: Set(invoice.description),
            ..Default::default()
        };

        let result = match txn {
            Some(txn) => model.update(txn).await,
            None => model.update(&self.db).await,
        };

        let model = result.map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(model.into())
    }

    async fn delete_many(&self, filter: InvoiceFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.id, |q, id| q.filter(Column::Id.eq(id)))
            .apply_if(filter.status, |q, status| match status {
                InvoiceStatus::PENDING => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).gt(Expr::current_timestamp()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
                InvoiceStatus::SETTLED => q.filter(Expr::col(Column::PaymentTime).is_not_null()),
                InvoiceStatus::EXPIRED => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).lte(Expr::current_timestamp()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
            })
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

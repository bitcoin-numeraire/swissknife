use crate::{
    application::{entities::Ledger, errors::DatabaseError},
    domains::invoices::entities::{Invoice, InvoiceFilter, InvoiceStatus},
};
use async_trait::async_trait;
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition,
    DatabaseConnection, DatabaseTransaction, EntityTrait, Order, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait,
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
        let order_direction: Order = filter.order_direction.into();

        let models = Entity::find()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.status, |q, s| match s {
                InvoiceStatus::Pending => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).gt(Expr::current_timestamp()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
                InvoiceStatus::Settled => q.filter(Expr::col(Column::PaymentTime).is_not_null()),
                InvoiceStatus::Expired => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).lte(Expr::current_timestamp()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
            })
            .apply_if(filter.ledger, |q, l| {
                q.filter(Column::Ledger.eq(l.to_string()))
            })
            .order_by(Column::PaymentTime, order_direction.clone())
            .order_by(Column::Timestamp, order_direction)
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
            ln_address: Set(invoice.ln_address),
            description: Set(invoice.description),
            amount_msat: Set(invoice.amount_msat.map(|v| v as i64)),
            timestamp: Set(invoice.timestamp),
            ledger: Set(invoice.ledger.to_string()),
            currency: Set(invoice.currency.to_string()),
            ..Default::default()
        };

        if invoice.ledger == Ledger::Lightning {
            let lightning = invoice
                .lightning
                .expect("should exist for ledger Lightning");
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
            amount_msat: Set(invoice.amount_msat.map(|v| v as i64)),
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
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.status, |q, status| match status {
                InvoiceStatus::Pending => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).gt(Expr::current_timestamp()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
                InvoiceStatus::Settled => q.filter(Expr::col(Column::PaymentTime).is_not_null()),
                InvoiceStatus::Expired => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).lte(Expr::current_timestamp()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
            })
            .apply_if(filter.ledger, |q, l| {
                q.filter(Column::Ledger.eq(l.to_string()))
            })
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

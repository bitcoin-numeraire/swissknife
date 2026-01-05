use crate::{
    application::{entities::Ledger, errors::DatabaseError},
    domains::invoice::{Invoice, InvoiceFilter, InvoiceOrderBy, InvoiceRepository, InvoiceStatus},
    infra::database::sea_orm::models::{
        btc_output,
        invoice::{ActiveModel, Column, Entity, Model as InvoiceModel},
    },
};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, Set, Unchanged,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct SeaOrmInvoiceRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmInvoiceRepository {
    fn map_with_output(model: InvoiceModel, btc_output: Option<btc_output::Model>) -> Invoice {
        let mut invoice: Invoice = model.into();

        if let Some(output) = btc_output {
            invoice.btc_output_id = Some(output.id);
            invoice.bitcoin_output = Some(output.into());
        }

        invoice
    }

    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl InvoiceRepository for SeaOrmInvoiceRepository {
    async fn find(&self, id: Uuid) -> Result<Option<Invoice>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .find_also_related(btc_output::Entity)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(|(invoice, btc_output)| Self::map_with_output(invoice, btc_output)))
    }

    async fn find_by_payment_hash(&self, payment_hash: &str) -> Result<Option<Invoice>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::PaymentHash.eq(payment_hash))
            .find_also_related(btc_output::Entity)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(|(invoice, btc_output)| Self::map_with_output(invoice, btc_output)))
    }

    async fn find_by_btc_output_id(&self, btc_output_id: Uuid) -> Result<Option<Invoice>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::BtcOutputId.eq(btc_output_id))
            .find_also_related(btc_output::Entity)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(|(invoice, btc_output)| Self::map_with_output(invoice, btc_output)))
    }

    async fn find_many(&self, filter: InvoiceFilter) -> Result<Vec<Invoice>, DatabaseError> {
        let order_by_column = match filter.order_by {
            InvoiceOrderBy::CreatedAt => Column::CreatedAt,
            InvoiceOrderBy::PaymentTime => Column::PaymentTime,
            InvoiceOrderBy::UpdatedAt => Column::UpdatedAt,
        };

        let models = Entity::find()
            .apply_if(filter.wallet_id, |q, wallet| q.filter(Column::WalletId.eq(wallet)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.status, |q, s| match s {
                InvoiceStatus::Pending => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).gt(Utc::now()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
                InvoiceStatus::Settled => q.filter(Expr::col(Column::PaymentTime).is_not_null()),
                InvoiceStatus::Expired => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).lte(Utc::now()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
            })
            .apply_if(filter.ledger, |q, l| q.filter(Column::Ledger.eq(l.to_string())))
            .order_by(order_by_column, filter.order_direction.into())
            .offset(filter.offset)
            .limit(filter.limit)
            .find_also_related(btc_output::Entity)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models
            .into_iter()
            .map(|(invoice, btc_output)| Self::map_with_output(invoice, btc_output))
            .collect())
    }

    async fn insert(&self, invoice: Invoice) -> Result<Invoice, DatabaseError> {
        let mut model = ActiveModel {
            id: Set(Uuid::new_v4()),
            wallet_id: Set(invoice.wallet_id),
            ln_address_id: Set(invoice.ln_address_id),
            description: Set(invoice.description),
            amount_msat: Set(invoice.amount_msat.map(|v| v as i64)),
            amount_received_msat: Set(invoice.amount_received_msat.map(|v| v as i64)),
            timestamp: Set(invoice.timestamp.naive_utc()),
            payment_time: Set(invoice.payment_time.map(|t| t.naive_utc())),
            ledger: Set(invoice.ledger.to_string()),
            currency: Set(invoice.currency.to_string()),
            btc_output_id: Set(invoice.btc_output_id),
            ..Default::default()
        };

        if invoice.ledger == Ledger::Lightning {
            let ln_invoice = invoice.ln_invoice.expect("should exist for ledger Lightning");
            model.bolt11 = Set(ln_invoice.bolt11.into());
            model.payee_pubkey = Set(ln_invoice.payee_pubkey.into());
            model.payment_hash = Set(ln_invoice.payment_hash.into());
            model.description_hash = Set(ln_invoice.description_hash);
            model.payment_secret = Set(ln_invoice.payment_secret.into());
            model.min_final_cltv_expiry_delta = Set((ln_invoice.min_final_cltv_expiry_delta as i64).into());
            model.expiry = Set((ln_invoice.expiry.as_secs() as i64).into());
            model.expires_at = Set(Some(ln_invoice.expires_at.naive_utc()));
        }

        let result = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(result.into())
    }

    async fn update(&self, invoice: Invoice) -> Result<Invoice, DatabaseError> {
        let model = ActiveModel {
            id: Unchanged(invoice.id),
            fee_msat: Set(invoice.fee_msat.map(|v| v as i64)),
            payment_time: Set(invoice.payment_time.map(|t| t.naive_utc())),
            description: Set(invoice.description),
            amount_received_msat: Set(invoice.amount_received_msat.map(|v| v as i64)),
            ledger: Set(invoice.ledger.to_string()),
            btc_output_id: Set(invoice.btc_output_id),
            updated_at: Set(Some(Utc::now().naive_utc())),
            ..Default::default()
        };

        let result = model
            .update(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.into())
    }

    async fn delete_many(&self, filter: InvoiceFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.wallet_id, |q, wallet| q.filter(Column::WalletId.eq(wallet)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.status, |q, status| match status {
                InvoiceStatus::Pending => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).gt(Utc::now()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
                InvoiceStatus::Settled => q.filter(Expr::col(Column::PaymentTime).is_not_null()),
                InvoiceStatus::Expired => q.filter(
                    Condition::all()
                        .add(Expr::col(Column::ExpiresAt).lte(Utc::now()))
                        .add(Expr::col(Column::PaymentTime).is_null()),
                ),
            })
            .apply_if(filter.ledger, |q, l| q.filter(Column::Ledger.eq(l.to_string())))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

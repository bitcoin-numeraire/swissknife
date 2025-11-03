use async_trait::async_trait;
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, ModelTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::{
        payment::PaymentStatus,
        wallet::{Balance, Contact, Wallet, WalletFilter, WalletOverview, WalletRepository},
    },
    infra::database::sea_orm::models::{
        contact::ContactModel,
        invoice::Column as InvoiceColumn,
        payment::Column as PaymentColumn,
        prelude::{Invoice, LnAddress, Payment},
        wallet::{ActiveModel, Column, Entity},
    },
};

#[derive(Clone)]
pub struct SeaOrmWalletRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmWalletRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl WalletRepository for SeaOrmWalletRepository {
    async fn find(&self, id: Uuid) -> Result<Option<Wallet>, DatabaseError> {
        let model_opt = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        match model_opt {
            Some(model) => {
                let balance = self.get_balance(None, id).await?;
                let payments = model
                    .find_related(Payment)
                    .all(&self.db)
                    .await
                    .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
                let invoices = model
                    .find_related(Invoice)
                    .all(&self.db)
                    .await
                    .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
                let ln_address = model
                    .find_related(LnAddress)
                    .one(&self.db)
                    .await
                    .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
                let contacts = self.find_contacts(id).await?;

                let mut wallet: Wallet = model.into();
                wallet.balance = balance;
                wallet.payments = payments.into_iter().map(Into::into).collect();
                wallet.invoices = invoices.into_iter().map(Into::into).collect();
                wallet.ln_address = ln_address.map(Into::into);
                wallet.contacts = contacts;

                return Ok(Some(wallet));
            }
            None => return Ok(None),
        };
    }

    async fn find_by_user_id(&self, user_id: &str) -> Result<Option<Wallet>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_many(&self, filter: WalletFilter) -> Result<Vec<Wallet>, DatabaseError> {
        let models = Entity::find()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .order_by(Column::CreatedAt, filter.order_direction.into())
            .offset(filter.offset)
            .limit(filter.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn find_many_overview(&self) -> Result<Vec<WalletOverview>, DatabaseError> {
        // Get all wallets with their ln_address (1-to-1 relation)
        let wallets_with_ln = Entity::find()
            .find_also_related(LnAddress)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        // Get invoice aggregates grouped by wallet_id
        let invoice_aggs = Invoice::find()
            .select_only()
            .column(InvoiceColumn::WalletId)
            .column_as(
                Expr::cust("CAST(SUM(invoice.amount_received_msat) AS BIGINT)"),
                "received_msat",
            )
            .column_as(InvoiceColumn::Id.count(), "n_invoices")
            .group_by(InvoiceColumn::WalletId)
            .into_tuple::<(Uuid, Option<i64>, i64)>()
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        // Get payment aggregates grouped by wallet_id
        let payment_aggs = Payment::find()
            .select_only()
            .column(PaymentColumn::WalletId)
            .column_as(
                Expr::cust("CAST(SUM(payment.amount_msat) AS BIGINT)"),
                "sent_msat",
            )
            .column_as(
                Expr::cust("CAST(SUM(payment.fee_msat) AS BIGINT)"),
                "fees_paid_msat",
            )
            .column_as(PaymentColumn::Id.count(), "n_payments")
            .column_as(Expr::col(PaymentColumn::LnAddress).count_distinct(), "n_contacts")
            .group_by(PaymentColumn::WalletId)
            .into_tuple::<(Uuid, Option<i64>, Option<i64>, i64, i64)>()
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        // Convert to HashMaps for efficient lookup
        let invoice_map: std::collections::HashMap<_, _> = invoice_aggs
            .into_iter()
            .map(|(id, received, count)| (id, (received, count)))
            .collect();
        let payment_map: std::collections::HashMap<_, _> = payment_aggs
            .into_iter()
            .map(|(id, sent, fees, count, contacts)| (id, (sent, fees, count, contacts)))
            .collect();

        // Combine the results
        Ok(wallets_with_ln
            .into_iter()
            .map(|(wallet_model, ln_address_model)| {
                let wallet_id = wallet_model.id;
                let (received_msat, n_invoices) =
                    invoice_map.get(&wallet_id).map(|(r, n)| (*r, *n)).unwrap_or((None, 0));
                let (sent_msat, fees_paid_msat, n_payments, n_contacts) = payment_map
                    .get(&wallet_id)
                    .map(|(s, f, np, nc)| (*s, *f, *np, *nc))
                    .unwrap_or((None, None, 0, 0));

                let received_msat_i64 = received_msat.unwrap_or(0);
                let sent_msat_i64 = sent_msat.unwrap_or(0);
                let fees_paid_msat_i64 = fees_paid_msat.unwrap_or(0);

                WalletOverview {
                    id: wallet_model.id,
                    user_id: wallet_model.user_id,
                    balance: Balance {
                        received_msat: received_msat_i64 as u64,
                        sent_msat: sent_msat_i64 as u64,
                        fees_paid_msat: fees_paid_msat_i64 as u64,
                        available_msat: received_msat_i64 - (sent_msat_i64 + fees_paid_msat_i64),
                    },
                    n_payments: n_payments as u32,
                    n_invoices: n_invoices as u32,
                    n_contacts: n_contacts as u32,
                    ln_address: ln_address_model.map(Into::into),
                    created_at: wallet_model.created_at.and_utc(),
                    updated_at: wallet_model.updated_at.map(|t| t.and_utc()),
                }
            })
            .collect())
    }

    async fn insert(&self, user_id: &str) -> Result<Wallet, DatabaseError> {
        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id.to_string()),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn get_balance(&self, txn: Option<&DatabaseTransaction>, id: Uuid) -> Result<Balance, DatabaseError> {
        let received = Invoice::find()
            .filter(InvoiceColumn::WalletId.eq(id))
            .select_only()
            .column_as(
                Expr::cust("CAST(SUM(invoice.amount_received_msat) AS BIGINT)"),
                "received_msat",
            )
            .into_tuple::<Option<i64>>();

        let sent = Payment::find()
            .filter(PaymentColumn::WalletId.eq(id))
            .filter(
                PaymentColumn::Status.is_in([PaymentStatus::Settled.to_string(), PaymentStatus::Pending.to_string()]),
            )
            .select_only()
            .column_as(
                Expr::cust("CAST(SUM(payment.amount_msat) AS BIGINT)"),
                "sent_msat",
            )
            .column_as(
                Expr::cust("CAST(SUM(payment.fee_msat) AS BIGINT)"),
                "fees_paid_msat",
            )
            .into_tuple::<(Option<i64>, Option<i64>)>();

        let (received_res, sent_res) = match txn {
            Some(txn) => (received.one(txn).await, sent.one(txn).await),
            None => (received.one(&self.db).await, sent.one(&self.db).await),
        };

        let received = received_res
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .unwrap_or(None);

        let (sent_msat, fees_paid_msat) = sent_res
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .unwrap_or((None, None));

        let received_msat_i64 = received.unwrap_or(0);
        let sent_msat_i64 = sent_msat.unwrap_or(0);
        let fees_paid_msat_i64 = fees_paid_msat.unwrap_or(0);

        Ok(Balance {
            received_msat: received_msat_i64 as u64,
            sent_msat: sent_msat_i64 as u64,
            fees_paid_msat: fees_paid_msat_i64 as u64,
            available_msat: received_msat_i64 - (sent_msat_i64 + fees_paid_msat_i64),
        })
    }

    async fn find_contacts(&self, id: Uuid) -> Result<Vec<Contact>, DatabaseError> {
        let models = Payment::find()
            .filter(PaymentColumn::WalletId.eq(id))
            .filter(PaymentColumn::LnAddress.is_not_null())
            .filter(PaymentColumn::Status.eq(PaymentStatus::Settled.to_string()))
            .select_only()
            .column(PaymentColumn::LnAddress)
            .column_as(PaymentColumn::PaymentTime.min(), "contact_since")
            .group_by(PaymentColumn::LnAddress)
            .order_by_asc(PaymentColumn::LnAddress)
            .into_model::<ContactModel>()
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        let response: Vec<Contact> = models.into_iter().map(Into::into).collect();
        Ok(response)
    }

    async fn delete_many(&self, filter: WalletFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

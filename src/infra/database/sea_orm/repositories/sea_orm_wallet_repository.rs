use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::{Expr, OnConflict},
    ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait, Set,
};
use uuid::Uuid;

use super::SeaOrmConnection;

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
        prelude::{Asset as AssetEntity, BtcAddress, BtcOutput, Invoice, LnAddress, Payment, Wallet as WalletEntity},
        wallet::{ActiveModel, Column},
    },
};

#[derive(Clone)]
pub struct SeaOrmWalletRepository<C = DatabaseConnection> {
    db: C,
}

impl<C> SeaOrmWalletRepository<C> {
    pub fn new(db: C) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<C> WalletRepository for SeaOrmWalletRepository<C>
where
    C: SeaOrmConnection,
{
    async fn find(&self, id: Uuid) -> Result<Option<Wallet>, DatabaseError> {
        let Some(model) = WalletEntity::find_by_id(id)
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
        else {
            return Ok(None);
        };

        let payments_with_output = Payment::find()
            .filter(PaymentColumn::WalletId.eq(id))
            .all(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
        let invoices_with_output = Invoice::find()
            .filter(InvoiceColumn::WalletId.eq(id))
            .find_also_related(BtcOutput)
            .all(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
        let ln_address = model
            .find_related(LnAddress)
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
        let btc_addresses = model
            .find_related(BtcAddress)
            .all(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
        let contacts = self.find_contacts(id).await?;
        let asset = AssetEntity::find_by_id(model.asset_id)
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        let mut wallet: Wallet = model.into();
        wallet.asset = asset.map(Into::into);
        wallet.balance = self.get_balance(wallet.id).await?;
        wallet.payments = payments_with_output.into_iter().map(Into::into).collect();
        wallet.invoices = invoices_with_output
            .into_iter()
            .map(|(invoice_model, output_model)| {
                let mut invoice: crate::domains::invoice::Invoice = invoice_model.into();
                invoice.bitcoin_output = output_model.map(Into::into);
                invoice
            })
            .collect();
        wallet.ln_address = ln_address.map(Into::into);
        wallet.btc_addresses = btc_addresses.into_iter().map(Into::into).collect();
        wallet.contacts = contacts;

        Ok(Some(wallet))
    }

    async fn find_by_account_and_asset(
        &self,
        account_id: Uuid,
        asset_id: Uuid,
    ) -> Result<Option<Wallet>, DatabaseError> {
        let model = WalletEntity::find()
            .filter(Column::AccountId.eq(account_id))
            .filter(Column::AssetId.eq(asset_id))
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        if let Some(model) = model {
            let asset = AssetEntity::find_by_id(model.asset_id)
                .one(self.db.connection())
                .await
                .map_err(|e| DatabaseError::FindOne(e.to_string()))?;
            let mut wallet: Wallet = model.into();
            wallet.asset = asset.map(Into::into);
            wallet.balance = self.get_balance(wallet.id).await?;
            Ok(Some(wallet))
        } else {
            Ok(None)
        }
    }

    async fn find_many(&self, filter: WalletFilter) -> Result<Vec<Wallet>, DatabaseError> {
        let models = WalletEntity::find()
            .apply_if(filter.account_id, |q, account_id| {
                q.filter(Column::AccountId.eq(account_id))
            })
            .apply_if(filter.asset_id, |q, asset_id| q.filter(Column::AssetId.eq(asset_id)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .order_by(
                Column::CreatedAt,
                crate::infra::database::sea_orm::sea_order(&filter.order_direction),
            )
            .offset(filter.offset)
            .limit(filter.limit)
            .all(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        let mut wallets = Vec::with_capacity(models.len());
        for model in models {
            let asset = AssetEntity::find_by_id(model.asset_id)
                .one(self.db.connection())
                .await
                .map_err(|e| DatabaseError::FindOne(e.to_string()))?;
            let mut wallet: Wallet = model.into();
            wallet.asset = asset.map(Into::into);
            wallet.balance = self.get_balance(wallet.id).await?;
            wallets.push(wallet);
        }

        Ok(wallets)
    }

    async fn find_many_overview(&self) -> Result<Vec<WalletOverview>, DatabaseError> {
        let wallets_with_ln = WalletEntity::find()
            .find_also_related(LnAddress)
            .all(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

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
            .all(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        let settled_payment_status = PaymentStatus::Settled.to_string();

        let payment_aggs = Payment::find()
            .select_only()
            .column(PaymentColumn::WalletId)
            .column_as(
                Expr::cust(format!(
                    "CAST(SUM(CASE WHEN payment.status = '{settled_status}' THEN payment.amount_msat ELSE 0 END) AS BIGINT)",
                    settled_status = settled_payment_status
                )),
                "sent_msat",
            )
            .column_as(
                Expr::cust(format!(
                    "CAST(SUM(CASE WHEN payment.status = '{settled_status}' THEN COALESCE(payment.fee_msat, 0) ELSE 0 END) AS BIGINT)",
                    settled_status = settled_payment_status
                )),
                "fees_paid_msat",
            )
            .column_as(PaymentColumn::Id.count(), "n_payments")
            .column_as(Expr::col(PaymentColumn::LnAddress).count_distinct(), "n_contacts")
            .group_by(PaymentColumn::WalletId)
            .into_tuple::<(Uuid, Option<i64>, Option<i64>, i64, i64)>()
            .all(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        let invoice_map: std::collections::HashMap<_, _> = invoice_aggs
            .into_iter()
            .map(|(id, received, count)| (id, (received, count)))
            .collect();
        let payment_map: std::collections::HashMap<_, _> = payment_aggs
            .into_iter()
            .map(|(id, sent, fees, count, contacts)| (id, (sent, fees, count, contacts)))
            .collect();

        let mut overviews = Vec::with_capacity(wallets_with_ln.len());
        for (wallet_model, ln_address_model) in wallets_with_ln {
            let wallet_id = wallet_model.id;
            let asset_model = AssetEntity::find_by_id(wallet_model.asset_id)
                .one(self.db.connection())
                .await
                .map_err(|e| DatabaseError::FindOne(e.to_string()))?;
            let (received_msat, n_invoices) = invoice_map.get(&wallet_id).map(|(r, n)| (*r, *n)).unwrap_or((None, 0));
            let (sent_msat, fees_paid_msat, n_payments, n_contacts) = payment_map
                .get(&wallet_id)
                .map(|(s, f, np, nc)| (*s, *f, *np, *nc))
                .unwrap_or((None, None, 0, 0));

            overviews.push(WalletOverview {
                id: wallet_model.id,
                account_id: wallet_model.account_id,
                asset_id: wallet_model.asset_id,
                asset: asset_model.map(Into::into),
                label: wallet_model.label,
                balance: Balance {
                    received_msat: received_msat.unwrap_or(0) as u64,
                    sent_msat: sent_msat.unwrap_or(0) as u64,
                    fees_paid_msat: fees_paid_msat.unwrap_or(0) as u64,
                    reserved_msat: wallet_model.reserved_amount as u64,
                    available_msat: wallet_model.available_amount,
                },
                n_payments: n_payments as u32,
                n_invoices: n_invoices as u32,
                n_contacts: n_contacts as u32,
                ln_address: ln_address_model.map(Into::into),
                created_at: wallet_model.created_at.and_utc(),
                updated_at: wallet_model.updated_at.map(|t| t.and_utc()),
            });
        }

        Ok(overviews)
    }

    async fn upsert(&self, account_id: Uuid, asset_id: Uuid) -> Result<Wallet, DatabaseError> {
        let id = Uuid::new_v4();
        let model = ActiveModel {
            id: Set(id),
            account_id: Set(account_id),
            asset_id: Set(asset_id),
            available_amount: Set(0),
            reserved_amount: Set(0),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        };

        WalletEntity::insert(model)
            .on_conflict(
                OnConflict::columns([Column::AccountId, Column::AssetId])
                    .do_nothing()
                    .to_owned(),
            )
            .exec_without_returning(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        self.find_by_account_and_asset(account_id, asset_id)
            .await?
            .ok_or_else(|| DatabaseError::Insert("wallet was not available after idempotent provisioning".to_string()))
    }

    async fn get_balance(&self, id: Uuid) -> Result<Balance, DatabaseError> {
        let wallet_amounts = WalletEntity::find_by_id(id)
            .select_only()
            .column(Column::AvailableAmount)
            .column(Column::ReservedAmount)
            .into_tuple::<(i64, i64)>()
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        let (available_msat, reserved_msat) = wallet_amounts.unwrap_or((0, 0));

        let received = Invoice::find()
            .filter(InvoiceColumn::WalletId.eq(id))
            .select_only()
            .column_as(
                Expr::cust("CAST(SUM(invoice.amount_received_msat) AS BIGINT)"),
                "received_msat",
            )
            .into_tuple::<Option<i64>>()
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .unwrap_or(None);

        let (sent_msat, fees_paid_msat) = Payment::find()
            .filter(PaymentColumn::WalletId.eq(id))
            .filter(PaymentColumn::Status.eq(PaymentStatus::Settled.to_string()))
            .select_only()
            .column_as(Expr::cust("CAST(SUM(payment.amount_msat) AS BIGINT)"), "sent_msat")
            .column_as(Expr::cust("CAST(SUM(payment.fee_msat) AS BIGINT)"), "fees_paid_msat")
            .into_tuple::<(Option<i64>, Option<i64>)>()
            .one(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .unwrap_or((None, None));

        Ok(Balance {
            received_msat: received.unwrap_or(0) as u64,
            sent_msat: sent_msat.unwrap_or(0) as u64,
            fees_paid_msat: fees_paid_msat.unwrap_or(0) as u64,
            reserved_msat: reserved_msat as u64,
            available_msat,
        })
    }

    async fn credit(&self, id: Uuid, amount_msat: u64) -> Result<(), DatabaseError> {
        let amount = amount_msat as i64;
        let result = WalletEntity::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).add(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::Id.eq(id))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        if result.rows_affected == 1 {
            Ok(())
        } else {
            Err(DatabaseError::Update(
                "wallet balance credit target was missing".to_string(),
            ))
        }
    }

    async fn reserve(&self, id: Uuid, amount_msat: u64) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = WalletEntity::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).sub(amount))
            .col_expr(Column::ReservedAmount, Expr::col(Column::ReservedAmount).add(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::Id.eq(id))
            .filter(Column::AvailableAmount.gte(amount))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
    }

    async fn debit(&self, id: Uuid, amount_msat: u64) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = WalletEntity::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).sub(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::Id.eq(id))
            .filter(Column::AvailableAmount.gte(amount))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
    }

    async fn debit_confirmed(&self, id: Uuid, amount_msat: u64) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = WalletEntity::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).sub(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::Id.eq(id))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
    }

    async fn release(&self, id: Uuid, amount_msat: u64) -> Result<bool, DatabaseError> {
        let amount = amount_msat as i64;
        let result = WalletEntity::update_many()
            .col_expr(Column::AvailableAmount, Expr::col(Column::AvailableAmount).add(amount))
            .col_expr(Column::ReservedAmount, Expr::col(Column::ReservedAmount).sub(amount))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
            .filter(Column::Id.eq(id))
            .filter(Column::ReservedAmount.gte(amount))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(result.rows_affected == 1)
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
            .all(self.db.connection())
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn delete_many(&self, filter: WalletFilter) -> Result<u64, DatabaseError> {
        let result = WalletEntity::delete_many()
            .apply_if(filter.account_id, |q, account_id| {
                q.filter(Column::AccountId.eq(account_id))
            })
            .apply_if(filter.asset_id, |q, asset_id| q.filter(Column::AssetId.eq(asset_id)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .exec(self.db.connection())
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

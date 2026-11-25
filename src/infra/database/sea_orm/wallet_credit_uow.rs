use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::OnConflict, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use swissknife_types::{CreateWalletCredit, Invoice, InvoiceStatus, Ledger, WalletCreditReceipt};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError},
    domains::{
        account::User,
        invoice::InvoiceRepository,
        wallet::{WalletCreditUnitOfWork, WalletRepository},
    },
};

use super::{
    amount_to_db,
    models::{prelude::WalletCredit, wallet, wallet_credit},
    NewClientEvent, SeaOrmClientEventRepository, SeaOrmInvoiceRepository, SeaOrmWalletRepository,
};

pub struct SeaOrmWalletCreditUnitOfWork {
    db: DatabaseConnection,
}

impl SeaOrmWalletCreditUnitOfWork {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

impl TryFrom<wallet_credit::Model> for WalletCreditReceipt {
    type Error = DataError;
    fn try_from(row: wallet_credit::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            request: CreateWalletCredit {
                request_id: row.request_id,
                account_id: row.target_account_id,
                asset_id: row.asset_id,
                amount_units: row
                    .amount_units
                    .try_into()
                    .map_err(|_| DataError::Inconsistency("Invalid stored credit amount".into()))?,
                purpose: row
                    .purpose
                    .parse()
                    .map_err(|_| DataError::Inconsistency("Invalid stored credit purpose".into()))?,
                reason: row.reason,
            },
            wallet_id: row.wallet_id,
            actor_account_id: row.actor_account_id,
            api_key_id: row.api_key_id,
            invoice_id: row.invoice_id,
            created_at: row.created_at.and_utc(),
        })
    }
}

#[async_trait]
impl WalletCreditUnitOfWork for SeaOrmWalletCreditUnitOfWork {
    async fn post(
        &self,
        wallet_id: Uuid,
        actor: &User,
        request: CreateWalletCredit,
    ) -> Result<WalletCreditReceipt, ApplicationError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        let amount = amount_to_db(request.amount_units)?;
        let invoice_id = Uuid::new_v4();
        let now = Utc::now();
        // Claim the key before reading. Concurrent SQLite writers and PostgreSQL
        // transactions serialize at the insert, including the unique opening-credit index.
        let inserted = WalletCredit::insert(wallet_credit::ActiveModel {
            request_id: Set(request.request_id),
            invoice_id: Set(invoice_id),
            wallet_id: Set(wallet_id),
            actor_account_id: Set(actor.account_id),
            api_key_id: Set(actor.api_key_id),
            target_account_id: Set(request.account_id),
            asset_id: Set(request.asset_id),
            amount_units: Set(amount),
            reason: Set(request.reason.clone()),
            purpose: Set(request.purpose.to_string()),
            created_at: Set(now.naive_utc()),
        })
        .on_conflict(OnConflict::new().do_nothing().to_owned())
        .exec_without_returning(&txn)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let receipt: WalletCreditReceipt = WalletCredit::find_by_id(request.request_id)
            .one(&txn)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .ok_or_else(|| DataError::Conflict("An opening credit has already been recorded for this wallet.".into()))?
            .try_into()?;
        if receipt.actor_account_id != actor.account_id || receipt.wallet_id != wallet_id || receipt.request != request
        {
            return Err(DataError::Conflict("This request ID belongs to a different credit.".into()).into());
        }
        if inserted > 0 {
            let target = wallet::Entity::find_by_id(wallet_id)
                .filter(wallet::Column::AccountId.eq(request.account_id))
                .filter(wallet::Column::AssetId.eq(request.asset_id))
                .lock_exclusive()
                .one(&txn)
                .await
                .map_err(|e| DatabaseError::FindOne(e.to_string()))?
                .ok_or_else(|| DataError::NotFound("No wallet matches the selected account and asset.".into()))?;
            if target
                .available_amount
                .checked_add(amount)
                .and_then(|available| available.checked_add(target.reserved_amount))
                .is_none()
            {
                return Err(
                    DataError::Validation("Credit would exceed the wallet balance storage range.".into()).into(),
                );
            }
            SeaOrmWalletRepository::new(&txn)
                .credit(wallet_id, request.amount_units)
                .await?;
            let invoice = SeaOrmInvoiceRepository::new(&txn)
                .insert(Invoice {
                    id: invoice_id,
                    wallet_id,
                    description: Some(request.reason),
                    amount_msat: Some(request.amount_units),
                    amount_received_msat: Some(request.amount_units),
                    fee_msat: Some(0),
                    timestamp: now,
                    payment_time: Some(now),
                    status: InvoiceStatus::Settled,
                    ledger: Ledger::Adjustment,
                    credit_purpose: Some(request.purpose),
                    ..Default::default()
                })
                .await?;
            SeaOrmClientEventRepository::new(&txn)
                .append_event(NewClientEvent::invoice_paid(&invoice))
                .await?;
        }
        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;
        if inserted > 0 {
            tracing::info!(actor = %actor.account_id, %wallet_id, amount_units = request.amount_units, %invoice_id, request_id = %request.request_id, "Wallet credit committed");
        }
        Ok(receipt)
    }
}

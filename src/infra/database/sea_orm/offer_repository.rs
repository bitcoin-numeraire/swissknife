use super::models::offer_receipt;
use super::models::prelude::{OfferCursor as Cursor, OfferReceipt as Receipt, WalletOffer as Entity};
use super::{
    amount_to_db,
    models::{
        offer_cursor as cursor,
        wallet_offer::{self as offer, Column},
    },
    payment_command::db_error,
    NewClientEvent, SeaOrmClientEventRepository, SeaOrmInvoiceRepository, SeaOrmWalletRepository,
};
use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{
        invoice::{Invoice, InvoiceRepository, InvoiceStatus},
        offer::{
            CreateOfferRequest, NodeOffer, OfferFilter, OfferRepository, OfferState, OfferUpdate, StoredOffer,
            WalletOffer,
        },
        wallet::WalletRepository,
    },
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    sea_query::{Expr, OnConflict},
    ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    QueryTrait, Set, TransactionTrait, TryInsertResult,
};
use uuid::Uuid;
pub struct SeaOrmOfferRepository(pub DatabaseConnection);

fn stored(m: offer::Model) -> Result<StoredOffer, ApplicationError> {
    Ok(StoredOffer {
        backend: m.backend,
        offer: WalletOffer {
            id: m.id,
            wallet_id: m.wallet_id,
            description: m.description,
            amount_msat: m.amount_msat.map(|v| v as u64),
            single_use: m.single_use,
            state: m
                .state
                .parse()
                .map_err(|_| DataError::Inconsistency("Invalid stored offer state".into()))?,
            provider_id: m.provider_id,
            bolt12: m.bolt12,
            created_at: m.created_at.and_utc(),
        },
    })
}
async fn credit_receipt(tx: &DatabaseTransaction, backend: &str, update: OfferUpdate) -> Result<(), ApplicationError> {
    let Some(provider) = update.local_offer_id else {
        return Ok(());
    };
    if update.status != "paid" {
        return Ok(());
    }
    let Some(owner) = Entity::find()
        .filter(Column::Backend.eq(backend))
        .filter(Column::ProviderId.eq(provider))
        .one(tx)
        .await
        .map_err(db_error)?
    else {
        return Ok(());
    };
    // Administrative history deletion must never make a settled receipt creditable again.
    let grant = Receipt::insert(offer_receipt::ActiveModel {
        payment_hash: Set(update.payment_hash.clone()),
        offer_id: Set(owner.id),
    })
    .on_conflict(
        OnConflict::column(offer_receipt::Column::PaymentHash)
            .do_nothing()
            .to_owned(),
    )
    .try_insert()
    .exec(tx)
    .await
    .map_err(db_error)?;
    if !matches!(grant, TryInsertResult::Inserted(_)) {
        return Ok(());
    }
    let invoices = SeaOrmInvoiceRepository::new(tx);
    let malformed = || DataError::Inconsistency("Core Lightning returned an incomplete paid offer invoice".into());
    let amount = update.amount_received_msat.filter(|v| *v > 0).ok_or_else(malformed)?;
    let when = update
        .paid_at
        .and_then(|v| i64::try_from(v).ok())
        .and_then(|v| DateTime::<Utc>::from_timestamp(v, 0))
        .ok_or_else(malformed)?;
    let created = update
        .invoice_created_at
        .and_then(|v| i64::try_from(v).ok())
        .and_then(|v| DateTime::<Utc>::from_timestamp(v, 0))
        .ok_or_else(malformed)?;
    let details = swissknife_types::OfferInvoice {
        offer_id: owner.id,
        bolt12: update.bolt12.ok_or_else(malformed)?,
        payment_hash: update.payment_hash,
        payer_note: update.invreq_payer_note,
    };
    let invoice = invoices
        .insert(Invoice {
            wallet_id: owner.wallet_id,
            description: Some(owner.description),
            offer_invoice: Some(details),
            amount_msat: update.amount_msat,
            amount_received_msat: Some(amount),
            payment_time: Some(when),
            timestamp: created,
            fee_msat: Some(0),
            ledger: swissknife_types::Ledger::Lightning,
            status: InvoiceStatus::Settled,
            ..Default::default()
        })
        .await?;
    SeaOrmWalletRepository::new(tx).credit(owner.wallet_id, amount).await?;
    SeaOrmClientEventRepository::new(tx)
        .append_event(NewClientEvent::invoice_paid(&invoice))
        .await?;
    if owner.single_use {
        Entity::update_many()
            .filter(Column::Id.eq(owner.id))
            .col_expr(Column::State, Expr::value(OfferState::Disabled.to_string()))
            .exec(tx)
            .await
            .map_err(db_error)?;
    }
    Ok(())
}
#[async_trait]
impl OfferRepository for SeaOrmOfferRepository {
    async fn register(
        &self,
        wallet: Uuid,
        backend: &str,
        r: &CreateOfferRequest,
    ) -> Result<(StoredOffer, bool), ApplicationError> {
        let inserted = Entity::insert(offer::ActiveModel {
            id: Set(Uuid::new_v4()),
            wallet_id: Set(wallet),
            request_key: Set(r.idempotency_key),
            backend: Set(backend.into()),
            description: Set(r.description.clone()),
            amount_msat: Set(r.amount_msat.map(amount_to_db).transpose()?),
            single_use: Set(r.single_use),
            state: Set(OfferState::Creating.to_string()),
            ..Default::default()
        })
        .on_conflict(
            OnConflict::columns([Column::WalletId, Column::RequestKey])
                .do_nothing()
                .to_owned(),
        )
        .try_insert()
        .exec(&self.0)
        .await
        .map_err(db_error)?;
        let row = Entity::find()
            .filter(Column::WalletId.eq(wallet))
            .filter(Column::RequestKey.eq(r.idempotency_key))
            .one(&self.0)
            .await
            .map_err(db_error)?
            .ok_or_else(|| DataError::NotFound("Offer registration disappeared".into()))?;
        Ok((stored(row)?, matches!(inserted, TryInsertResult::Inserted(_))))
    }
    async fn owner(&self, provider_id: &str) -> Result<Option<Uuid>, ApplicationError> {
        Ok(Entity::find()
            .filter(Column::ProviderId.eq(provider_id))
            .select_only()
            .column(Column::WalletId)
            .into_tuple()
            .one(&self.0)
            .await
            .map_err(db_error)?)
    }
    async fn get(&self, id: Uuid, wallet: Option<Uuid>) -> Result<StoredOffer, ApplicationError> {
        let m = Entity::find_by_id(id)
            .apply_if(wallet, |q, w| q.filter(Column::WalletId.eq(w)))
            .one(&self.0)
            .await
            .map_err(db_error)?
            .ok_or_else(|| DataError::NotFound("Offer not found".into()))?;
        stored(m)
    }
    async fn list(&self, filter: OfferFilter) -> Result<Vec<WalletOffer>, ApplicationError> {
        Entity::find()
            .apply_if(filter.wallet_id, |q, w| q.filter(Column::WalletId.eq(w)))
            .order_by_desc(Column::CreatedAt)
            .order_by_desc(Column::Id)
            .limit(filter.limit.unwrap_or(50).min(200))
            .offset(filter.offset.unwrap_or(0))
            .all(&self.0)
            .await
            .map_err(db_error)?
            .into_iter()
            .map(|r| stored(r).map(|s| s.offer))
            .collect()
    }
    async fn activate(&self, id: Uuid, node: &NodeOffer) -> Result<StoredOffer, ApplicationError> {
        Entity::update_many()
            .filter(Column::Id.eq(id))
            .filter(Column::State.eq("creating"))
            .col_expr(
                Column::State,
                Expr::value(if node.active { "active" } else { "disabled" }),
            )
            .col_expr(Column::ProviderId, Expr::value(node.offer_id.clone()))
            .col_expr(Column::Bolt12, Expr::value(node.bolt12.clone()))
            .exec(&self.0)
            .await
            .map_err(db_error)?;
        self.get(id, None).await
    }
    async fn transition(&self, id: Uuid, from: OfferState, to: OfferState) -> Result<StoredOffer, ApplicationError> {
        Entity::update_many()
            .filter(Column::Id.eq(id))
            .filter(Column::State.eq(from.to_string()))
            .col_expr(Column::State, Expr::value(to.to_string()))
            .exec(&self.0)
            .await
            .map_err(db_error)?;
        self.get(id, None).await
    }
    async fn pending(&self, backend: &str) -> Result<Option<StoredOffer>, ApplicationError> {
        let eligible = (Utc::now() - chrono::Duration::seconds(30)).naive_utc();
        Entity::find()
            .filter(Column::Backend.eq(backend))
            .filter(Column::State.is_in(["creating", "disabling"]))
            .filter(Column::CreatedAt.lt(eligible))
            .order_by_asc(Column::CreatedAt)
            .one(&self.0)
            .await
            .map_err(db_error)?
            .map(stored)
            .transpose()
    }
    async fn cursor(&self, backend: &str) -> Result<Option<u64>, ApplicationError> {
        if Entity::find()
            .filter(Column::Backend.eq(backend))
            .filter(Column::ProviderId.is_not_null())
            .one(&self.0)
            .await
            .map_err(db_error)?
            .is_none()
        {
            return Ok(None);
        }
        Ok(Some(
            Cursor::find_by_id(backend)
                .one(&self.0)
                .await
                .map_err(db_error)?
                .map_or(0, |v| v.last_index as u64),
        ))
    }
    async fn project(&self, backend: &str, after: u64, updates: Vec<OfferUpdate>) -> Result<(), ApplicationError> {
        let mut last = after;
        for item in &updates {
            if item.updated_index <= last {
                return Err(
                    DataError::Inconsistency("Core Lightning returned unordered invoice updates".into()).into(),
                );
            }
            last = item.updated_index;
        }
        let txn = self.0.begin().await.map_err(db_error)?;
        Cursor::insert(cursor::ActiveModel {
            backend: Set(backend.into()),
            last_index: Set(0),
        })
        .on_conflict(OnConflict::column(cursor::Column::Backend).do_nothing().to_owned())
        .try_insert()
        .exec(&txn)
        .await
        .map_err(db_error)?;
        // A replica wins the entire page. Cursor, invoice history, balance and inbox commit together.
        let won = Cursor::update_many()
            .filter(cursor::Column::Backend.eq(backend))
            .filter(cursor::Column::LastIndex.eq(amount_to_db(after)?))
            .col_expr(cursor::Column::LastIndex, Expr::value(amount_to_db(last)?))
            .exec(&txn)
            .await
            .map_err(db_error)?;
        if won.rows_affected == 1 {
            for update in updates {
                credit_receipt(&txn, backend, update).await?;
            }
        }
        txn.commit().await.map_err(db_error)?;
        Ok(())
    }
}

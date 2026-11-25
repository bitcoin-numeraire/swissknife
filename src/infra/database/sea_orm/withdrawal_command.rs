use chrono::{Duration, Utc};
use sea_orm::{
    sea_query::Expr, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use swissknife_types::{PaymentExecutionState as State, WithdrawalSubmission};
use uuid::Uuid;

use super::{
    models::{
        payment,
        prelude::WithdrawalCommand as Entity,
        withdrawal_command::{ActiveModel, Column, Model},
    },
    payment_command::{append, db_error},
    uow::fail_in_transaction,
    SeaOrmPaymentRepository,
};
use crate::{
    application::composition::Ledger,
    application::errors::{ApplicationError, DataError, DatabaseError},
    domains::{
        event::ClientEventType,
        payment::{
            BtcPayment, ClaimedWithdrawal, Payment, PaymentRepository, PaymentStatus, StoredWithdrawal,
            WithdrawalProgress,
        },
    },
};

const CLAIM_SECONDS: i64 = 60;

fn progress(row: &Model) -> Result<WithdrawalProgress, DatabaseError> {
    serde_json::from_value(row.progress.clone())
        .map_err(|e| DatabaseError::FindOne(format!("Invalid withdrawal progress: {e}")))
}

fn details(row: &Model) -> WithdrawalSubmission {
    WithdrawalSubmission {
        awaiting_original_provider: false,
        address: row.address.clone(),
        maximum_fee_msat: row.maximum_fee_msat as u64,
        expires_at: row.expires_at.and_utc(),
    }
}

pub(super) async fn summary(
    db: &DatabaseConnection,
    payment: &Payment,
) -> Result<Option<StoredWithdrawal>, ApplicationError> {
    if payment.ledger != Ledger::Onchain {
        return Ok(None);
    }
    let Some(row) = Entity::find_by_id(payment.id).one(db).await.map_err(db_error)? else {
        return Ok(None);
    };
    Ok(Some(StoredWithdrawal {
        state: progress(&row)?.state(),
        details: details(&row),
        backend: row.backend,
    }))
}

pub(super) async fn insert(
    tx: &DatabaseTransaction,
    id: Uuid,
    backend: String,
    address: String,
    maximum_fee_msat: u64,
) -> Result<WithdrawalSubmission, ApplicationError> {
    let now = Utc::now();
    let expires_at = now + Duration::hours(1);
    Entity::insert(ActiveModel {
        payment_id: Set(id),
        backend: Set(backend),
        address: Set(address.clone()),
        maximum_fee_msat: Set(super::amount_to_db(maximum_fee_msat)?),
        expires_at: Set(expires_at.naive_utc()),
        progress: Set(serde_json::to_value(WithdrawalProgress::Accepted).expect("withdrawal serializes")),
        claim_token: Set(None),
        started_at: Set(None),
        next_attempt: Set(Some(now.naive_utc())),
    })
    .exec(tx)
    .await
    .map_err(db_error)?;
    Ok(WithdrawalSubmission {
        awaiting_original_provider: false,
        address,
        maximum_fee_msat,
        expires_at,
    })
}

pub(super) async fn claim(
    db: &DatabaseConnection,
    backend: &str,
) -> Result<Option<ClaimedWithdrawal>, ApplicationError> {
    let now = Utc::now();
    let Some(row) = Entity::find()
        .inner_join(payment::Entity)
        .filter(
            sea_orm::Condition::any().add(Column::Backend.eq(backend)).add(
                sea_orm::Condition::all()
                    .add(Column::ExpiresAt.lte(now.naive_utc()))
                    .add(payment::Column::PaymentHash.is_null()),
            ),
        )
        .filter(Column::NextAttempt.lte(now.naive_utc()))
        .filter(payment::Column::Status.eq("Pending"))
        .order_by_asc(Column::NextAttempt)
        .order_by_asc(Column::PaymentId)
        .one(db)
        .await
        .map_err(db_error)?
    else {
        return Ok(None);
    };
    let token = Uuid::new_v4();
    let tx = db.begin().await.map_err(db_error)?;
    let winner = Entity::update_many()
        .col_expr(Column::ClaimToken, Expr::value(token))
        .col_expr(
            Column::StartedAt,
            Expr::value(row.started_at.unwrap_or_else(|| now.naive_utc())),
        )
        .col_expr(
            Column::NextAttempt,
            Expr::value((now + Duration::seconds(CLAIM_SECONDS)).naive_utc()),
        )
        .filter(Column::PaymentId.eq(row.payment_id))
        .filter(Column::NextAttempt.eq(row.next_attempt))
        .exec(&tx)
        .await
        .map_err(db_error)?;
    if winner.rows_affected == 0 {
        return Ok(None);
    }
    let payment = SeaOrmPaymentRepository::new(&tx)
        .find(row.payment_id)
        .await?
        .ok_or_else(|| DataError::NotFound("Withdrawal payment not found".into()))?;
    if payment.status != PaymentStatus::Pending {
        return Ok(None);
    }
    let claim = ClaimedWithdrawal {
        payment,
        token,
        original_backend: row.backend == backend,
        details: details(&row),
        progress: progress(&row)?,
    };
    tx.commit().await.map_err(db_error)?;
    Ok(Some(claim))
}

pub(super) async fn advance(
    db: &DatabaseConnection,
    claim: &ClaimedWithdrawal,
    next: WithdrawalProgress,
    retry_seconds: i64,
) -> Result<bool, ApplicationError> {
    let tx = db.begin().await.map_err(db_error)?;
    let next = if matches!(&next, WithdrawalProgress::Signed { .. })
        && !matches!(claim.progress, WithdrawalProgress::Signed { .. })
        && Utc::now() >= claim.details.expires_at
    {
        WithdrawalProgress::Expired
    } else {
        next
    };
    let due = next
        .failure_reason()
        .is_none()
        .then(|| (Utc::now() + Duration::seconds(retry_seconds)).naive_utc());
    let updated = Entity::update_many()
        .col_expr(
            Column::Progress,
            Expr::value(serde_json::to_value(&next).expect("withdrawal serializes")),
        )
        .col_expr(Column::NextAttempt, Expr::value(due))
        .col_expr(Column::ClaimToken, Expr::value(Option::<Uuid>::None))
        .filter(Column::PaymentId.eq(claim.payment.id))
        .filter(Column::ClaimToken.eq(claim.token))
        .exec(&tx)
        .await
        .map_err(db_error)?;
    if updated.rows_affected == 0 {
        return Ok(false);
    }
    if !claim.progress.permits(&next) {
        return Err(DataError::Inconsistency("Invalid withdrawal execution transition".into()).into());
    }
    let payments = SeaOrmPaymentRepository::new(&tx);
    // Serialize progress events with settlement, including confirmation before the broadcast RPC replies.
    if !payments
        .try_transition(claim.payment.id, &[PaymentStatus::Pending], PaymentStatus::Pending)
        .await?
    {
        return Ok(false);
    }
    let mut payment = payments
        .find(claim.payment.id)
        .await?
        .ok_or_else(|| DataError::NotFound("Withdrawal disappeared".into()))?;
    if let Some(reason) = next.failure_reason() {
        payment.status = PaymentStatus::Failed;
        payment.error = Some(reason.to_owned());
        fail_in_transaction(&tx, payment).await?;
    } else {
        if let WithdrawalProgress::Signed {
            transaction, fee_msat, ..
        } = &next
        {
            if payment.bitcoin.is_none() {
                if *fee_msat > claim.details.maximum_fee_msat {
                    return Err(DataError::Inconsistency("Withdrawal exceeds approved fee".into()).into());
                }
                payment.bitcoin = Some(BtcPayment {
                    address: claim.details.address.clone(),
                    txid: transaction.txid(),
                    block_height: None,
                });
                payment.fee_msat = Some(*fee_msat);
                payment = payments.update(payment).await?;
            }
        }
        if next.state() != claim.progress.state() {
            let event = match next.state() {
                State::Processing => ClientEventType::WithdrawalProcessing,
                State::Waiting => ClientEventType::WithdrawalWaiting,
                State::Broadcast => ClientEventType::WithdrawalBroadcast,
                State::Reconciling => ClientEventType::WithdrawalReconciling,
                _ => return Err(DataError::Inconsistency("Unexpected withdrawal transition".into()).into()),
            };
            append(&tx, &payment, event).await?;
        }
    }
    tx.commit().await.map_err(db_error)?;
    Ok(true)
}

/// Settlement and workers take the command row before the payment row in the same order.
pub(super) async fn confirm(tx: &DatabaseTransaction, id: Uuid) -> Result<(), DatabaseError> {
    Entity::update_many()
        .col_expr(
            Column::Progress,
            Expr::value(serde_json::to_value(WithdrawalProgress::Confirmed).expect("withdrawal serializes")),
        )
        .col_expr(Column::NextAttempt, Expr::value(Option::<chrono::NaiveDateTime>::None))
        .col_expr(Column::ClaimToken, Expr::value(Option::<Uuid>::None))
        .filter(Column::PaymentId.eq(id))
        .filter(Column::NextAttempt.is_not_null())
        .exec(tx)
        .await
        .map_err(db_error)?;
    Ok(())
}

pub(super) async fn cancel(db: &DatabaseConnection, wallet_id: Option<Uuid>, id: Uuid) -> Result<(), ApplicationError> {
    let mut query = Entity::find_by_id(id).inner_join(payment::Entity);
    if let Some(wallet_id) = wallet_id {
        query = query.filter(payment::Column::WalletId.eq(wallet_id));
    }
    let row = query
        .one(db)
        .await
        .map_err(db_error)?
        .ok_or_else(|| DataError::NotFound("Withdrawal submission not found".into()))?;
    if matches!(progress(&row)?, WithdrawalProgress::Cancelled) {
        return Ok(());
    }
    let tx = db.begin().await.map_err(db_error)?;
    let cancelled = WithdrawalProgress::Cancelled;
    let updated = Entity::update_many()
        .col_expr(
            Column::Progress,
            Expr::value(serde_json::to_value(&cancelled).expect("withdrawal serializes")),
        )
        .col_expr(Column::NextAttempt, Expr::value(Option::<chrono::NaiveDateTime>::None))
        .filter(Column::PaymentId.eq(id))
        .filter(Column::StartedAt.is_null())
        .filter(Column::NextAttempt.is_not_null())
        .exec(&tx)
        .await
        .map_err(db_error)?;
    if updated.rows_affected == 0 {
        return Err(DataError::Conflict("Withdrawal execution has already started".into()).into());
    }
    let mut payment = SeaOrmPaymentRepository::new(&tx)
        .find(id)
        .await?
        .ok_or_else(|| DataError::NotFound("Withdrawal disappeared".into()))?;
    payment.status = PaymentStatus::Failed;
    payment.error = cancelled.failure_reason().map(str::to_owned);
    fail_in_transaction(&tx, payment).await?;
    tx.commit().await.map_err(db_error)?;
    Ok(())
}

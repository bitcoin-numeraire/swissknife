use super::{
    models::{
        agent_payment_approval::{ActiveModel, Column, Model},
        prelude::AgentPaymentApproval as Entity,
        spending_policy, wallet, wallet_agent_key,
    },
    payment_command::{db_error, Admission},
};
use crate::{
    application::errors::{ApplicationError, AuthenticationError, DataError, DatabaseError},
    domains::{
        agent::ReviewedPayment,
        payment::{Payment, PaymentInstructions, SubmissionKey},
    },
};
use chrono::{Duration, Utc};
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use swissknife_types::{ApprovalDecision, ApprovalState, PaymentApproval};
use uuid::Uuid;

fn terms(row: &Model) -> Result<ReviewedPayment, ApplicationError> {
    serde_json::from_value(row.intent.clone())
        .map_err(|e| DatabaseError::FindOne(format!("Invalid stored payment approval terms: {e}")).into())
}

fn state(row: &Model) -> Result<ApprovalState, ApplicationError> {
    match row.state.as_str() {
        "pending" | "approved" if row.expires_at <= Utc::now().naive_utc() => Ok(ApprovalState::Expired),
        "pending" => Ok(ApprovalState::Pending),
        "approved" => Ok(ApprovalState::Approved),
        "rejected" => Ok(ApprovalState::Rejected),
        "consumed" => Ok(ApprovalState::Consumed),
        _ => Err(DataError::Inconsistency("Unknown payment approval state".into()).into()),
    }
}

fn summary(row: Model) -> Result<PaymentApproval, ApplicationError> {
    let intent = terms(&row)?;
    Ok(PaymentApproval {
        id: row.id,
        wallet_id: row.wallet_id,
        agent_key_id: row.agent_key_id,
        idempotency_key: row.idempotency_key,
        requested_input: intent.requested_input,
        destination: intent.destination,
        ledger: intent.ledger,
        amount_msat: intent.amount_msat.to_string(),
        maximum_fee_msat: intent.maximum_fee_msat.to_string(),
        state: state(&row)?,
        created_at: row.created_at.and_utc(),
        expires_at: row.expires_at.and_utc(),
        decided_at: row.decided_at.map(|d| d.and_utc()),
        decided_by: row.decided_by,
    })
}

pub(super) async fn pending(db: &DatabaseConnection, account: Uuid) -> Result<Vec<PaymentApproval>, ApplicationError> {
    Entity::find()
        .filter(Column::AccountId.eq(account))
        .filter(Column::State.eq("pending"))
        .filter(Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(db)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(summary)
        .collect()
}
pub(super) async fn get(db: &DatabaseConnection, account: Uuid, id: Uuid) -> Result<PaymentApproval, ApplicationError> {
    let row = Entity::find_by_id(id)
        .filter(Column::AccountId.eq(account))
        .one(db)
        .await
        .map_err(db_error)?
        .ok_or_else(|| DataError::NotFound("Payment approval not found".into()))?;
    summary(row)
}
pub(super) async fn decide(
    db: &DatabaseConnection,
    account: Uuid,
    id: Uuid,
    operator: Uuid,
    decision: ApprovalDecision,
) -> Result<PaymentApproval, ApplicationError> {
    let verdict = match decision {
        ApprovalDecision::Approve => "approved",
        ApprovalDecision::Reject => "rejected",
    };
    let now = Utc::now().naive_utc();
    Entity::update_many()
        .col_expr(Column::State, Expr::value(verdict))
        .col_expr(Column::DecidedAt, Expr::value(now))
        .col_expr(Column::DecidedBy, Expr::value(operator))
        .filter(Column::Id.eq(id))
        .filter(Column::AccountId.eq(account))
        .filter(Column::State.eq("pending"))
        .filter(Column::ExpiresAt.gt(now))
        .exec(db)
        .await
        .map_err(db_error)?;
    let result = get(db, account, id).await?;
    if matches!(
        (decision, result.state),
        (
            ApprovalDecision::Approve,
            ApprovalState::Approved | ApprovalState::Consumed
        ) | (ApprovalDecision::Reject, ApprovalState::Rejected)
    ) {
        return Ok(result);
    }
    Err(DataError::Conflict("This payment request has expired or already has another decision".into()).into())
}

async fn find(db: &impl ConnectionTrait, key: &SubmissionKey) -> Result<Option<Model>, ApplicationError> {
    Ok(Entity::find()
        .filter(Column::WalletId.eq(key.wallet_id))
        .filter(Column::IdempotencyKey.eq(key.key))
        .one(db)
        .await
        .map_err(db_error)?)
}
fn binding(row: &Model, key: &SubmissionKey) -> Result<(), ApplicationError> {
    if row.fingerprint != key.fingerprint || Some(row.agent_key_id) != key.agent_key_id {
        return Err(DataError::Conflict("Idempotency key belongs to a different payment or credential".into()).into());
    }
    Ok(())
}
fn awaiting(row: &Model) -> DataError {
    DataError::ApprovalRequired {
        account: row.account_id,
        approval: row.id,
    }
}

pub(super) async fn reviewed(
    db: &DatabaseConnection,
    key: &SubmissionKey,
) -> Result<Option<ReviewedPayment>, ApplicationError> {
    let Some(row) = find(db, key).await? else {
        return Ok(None);
    };
    binding(&row, key)?;
    match state(&row)? {
        ApprovalState::Approved => Ok(Some(terms(&row)?)),
        ApprovalState::Consumed => Ok(None),
        ApprovalState::Pending => Err(awaiting(&row).into()),
        ApprovalState::Rejected => Err(DataError::Conflict("The operator rejected this payment".into()).into()),
        ApprovalState::Expired => Err(DataError::Conflict("This payment approval expired".into()).into()),
    }
}

fn resolved(
    key: &SubmissionKey,
    payment: &Payment,
    admission: &Admission,
) -> Result<ReviewedPayment, ApplicationError> {
    let lightning = payment.lightning.as_ref();
    let (input, destination, fee, recipient) = match admission {
        Admission::External {
            instructions:
                PaymentInstructions::Lightning {
                    invoice,
                    maximum_fee_msat,
                    ..
                },
            ..
        } => {
            let hash = &lightning
                .ok_or_else(|| DataError::Inconsistency("Lightning command is missing its payment hash".into()))?
                .payment_hash;
            (invoice, hash, *maximum_fee_msat, None)
        }
        Admission::External {
            instructions:
                PaymentInstructions::Onchain {
                    address,
                    maximum_fee_msat,
                    ..
                },
            ..
        } => (address, address, *maximum_fee_msat, None),
        Admission::Internal(invoice) => {
            let address = payment
                .internal
                .as_ref()
                .and_then(|i| i.btc_address.as_ref())
                .ok_or_else(|| DataError::Inconsistency("Internal withdrawal is missing its destination".into()))?;
            (address, address, payment.fee_msat.unwrap_or(0), Some(invoice.wallet_id))
        }
    };
    Ok(ReviewedPayment {
        requested_input: key.requested_input.clone(),
        input: input.clone(),
        destination: destination.clone(),
        ledger: payment.ledger.clone(),
        withdrawal_backend: match admission {
            Admission::External {
                instructions: PaymentInstructions::Onchain { backend, .. },
                ..
            } => Some(backend.clone()),
            _ => None,
        },
        recipient_wallet: recipient,
        amount_msat: payment.amount_msat,
        maximum_fee_msat: fee,
        description: payment.description.clone(),
        ln_address: lightning.and_then(|l| l.ln_address.clone()),
        success_action: lightning.and_then(|l| l.raw_success_action.clone()),
    })
}

/// Called after claiming the unique command key and before funds or ledger history move.
/// Return an approval error to commit only the review row. An approved request is consumed
/// in the same transaction as the eventual payment, so a failed budget check can be retried.
pub(super) async fn gate(
    tx: &DatabaseTransaction,
    key: &SubmissionKey,
    payment: &Payment,
    admission: &Admission,
) -> Result<Option<DataError>, ApplicationError> {
    let Some(agent) = key.agent_key_id else {
        if let Some(existing) = find(tx, key).await? {
            binding(&existing, key)?;
        }
        return Ok(None);
    };
    // Wallet, then policy matches the ordinary reservation lock order. The command write
    // already obtained SQLite's writer lock. The account policy serializes queue capacity.
    let account = wallet::Entity::find_by_id(key.wallet_id)
        .select_only()
        .column(wallet::Column::AccountId)
        // Concurrent command inserts hold KEY SHARE through the wallet foreign key.
        .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
        .into_tuple::<Uuid>()
        .one(tx)
        .await
        .map_err(db_error)?
        .ok_or_else(|| DataError::NotFound("Wallet not found".into()))?;
    let policy = spending_policy::Entity::find_by_id(account)
        .lock_exclusive()
        .one(tx)
        .await
        .map_err(db_error)?;
    let existing = find(tx, key).await?;
    let amount = match admission {
        Admission::External { reserve, .. } => *reserve,
        Admission::Internal(_) => payment
            .amount_msat
            .checked_add(payment.fee_msat.unwrap_or(0))
            .ok_or_else(|| DataError::Validation("Payment exceeds the ledger range".into()))?,
    };
    let cap = policy.filter(|p| p.enabled).and_then(|p| p.approval_above_msat);
    if existing.is_none() && cap.is_none_or(|cap| amount <= cap as u64) {
        return Ok(None);
    }
    let now = Utc::now();
    let credential = wallet_agent_key::Entity::find_by_id(agent)
        .filter(wallet_agent_key::Column::AccountId.eq(account))
        .filter(wallet_agent_key::Column::WalletId.eq(key.wallet_id))
        .filter(wallet_agent_key::Column::CanSpend.eq(true))
        .filter(wallet_agent_key::Column::ExpiresAt.gt(now.naive_utc()))
        .lock_shared()
        .one(tx)
        .await
        .map_err(db_error)?
        .ok_or(AuthenticationError::InvalidCredentials)?;
    let intent = resolved(key, payment, admission)?;
    if let Some(row) = existing {
        binding(&row, key)?;
        match state(&row)? {
            ApprovalState::Pending => return Ok(Some(awaiting(&row))),
            ApprovalState::Approved if terms(&row)?.allows(&intent) => {
                let changed = Entity::update_many()
                    .col_expr(Column::State, Expr::value("consumed"))
                    .filter(Column::Id.eq(row.id))
                    .filter(Column::State.eq("approved"))
                    .filter(Column::ExpiresAt.gt(Utc::now().naive_utc()))
                    .exec(tx)
                    .await
                    .map_err(db_error)?;
                if changed.rows_affected == 1 {
                    return Ok(None);
                }
            }
            _ => (),
        }
        return Err(DataError::Conflict(
            "Approval expired, was rejected, or does not cover these payment terms".into(),
        )
        .into());
    }
    let live = Entity::find()
        .filter(Column::AccountId.eq(account))
        .filter(Column::State.is_in(["pending", "approved"]))
        .filter(Column::ExpiresAt.gt(now.naive_utc()))
        .count(tx)
        .await
        .map_err(db_error)?;
    if live >= 100 {
        return Err(DataError::Conflict("The account has 100 outstanding payment approvals".into()).into());
    }
    let mut expires = (now + Duration::hours(24)).naive_utc().min(credential.expires_at);
    if let Ok(invoice) = intent.input.parse::<lightning_invoice::Bolt11Invoice>() {
        if let Some(end) = invoice
            .expires_at()
            .and_then(|v| i64::try_from(v.as_secs()).ok())
            .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
        {
            expires = expires.min(end.naive_utc());
        }
    }
    let id = Uuid::new_v4();
    ActiveModel {
        id: Set(id),
        wallet_id: Set(key.wallet_id),
        account_id: Set(account),
        agent_key_id: Set(agent),
        idempotency_key: Set(key.key),
        fingerprint: Set(key.fingerprint.clone()),
        intent: Set(serde_json::to_value(intent).map_err(|e| DatabaseError::Insert(e.to_string()))?),
        state: Set("pending".into()),
        created_at: Set(now.naive_utc()),
        expires_at: Set(expires),
        decided_at: Set(None),
        decided_by: Set(None),
    }
    .insert(tx)
    .await
    .map_err(db_error)?;
    Ok(Some(DataError::ApprovalRequired { account, approval: id }))
}

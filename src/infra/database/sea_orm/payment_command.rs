use super::models::prelude::PaymentCommand as Entity;
use chrono::Utc;
use sea_orm::{
    sea_query::{Expr, OnConflict},
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait, TryInsertResult,
};
use uuid::Uuid;

use super::{
    models::payment_command::{ActiveModel, Column, Model},
    uow::reserve_in_transaction,
    NewClientEvent, SeaOrmClientEventRepository, SeaOrmPaymentRepository,
};
use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError},
    domains::{
        event::ClientEventType,
        payment::{ClaimedPayment, Payment, PaymentInstructions, PaymentRepository, SubmissionKey, SubmittedPayment},
    },
};

fn db_error(error: sea_orm::DbErr) -> DatabaseError {
    DatabaseError::Transaction(error.to_string())
}

async fn find<C: ConnectionTrait>(db: &C, key: &SubmissionKey) -> Result<Option<Model>, ApplicationError> {
    Ok(Entity::find()
        .filter(Column::WalletId.eq(key.wallet_id))
        .filter(Column::IdempotencyKey.eq(key.key))
        .one(db)
        .await
        .map_err(db_error)?)
}

fn original_payment_id(row: &Model, key: &SubmissionKey) -> Result<Uuid, ApplicationError> {
    if row.request_fingerprint != key.fingerprint {
        return Err(DataError::Conflict("Idempotency key already used for a different payment".into()).into());
    }
    row.payment_id.ok_or_else(|| {
        DataError::Conflict("This payment was already submitted and its history was deleted".into()).into()
    })
}

pub(super) async fn submitted(
    db: &DatabaseConnection,
    key: &SubmissionKey,
) -> Result<Option<SubmittedPayment>, ApplicationError> {
    let Some(row) = find(db, key).await? else {
        return Ok(None);
    };
    let id = original_payment_id(&row, key)?;
    let payment = SeaOrmPaymentRepository::new(db.clone())
        .find(id)
        .await?
        .ok_or_else(|| DataError::Conflict("The submitted payment was deleted".into()))?;
    Ok(Some(SubmittedPayment {
        payment,
        dispatched: row.dispatched_at.is_some(),
    }))
}

async fn append(tx: &DatabaseTransaction, payment: &Payment, event_type: ClientEventType) -> Result<(), DatabaseError> {
    SeaOrmClientEventRepository::new(tx)
        .append_event(NewClientEvent {
            event_type,
            wallet_id: payment.wallet_id,
            resource_id: payment.id,
            data: serde_json::to_value(payment).expect("payment serializes"),
        })
        .await
}

pub(super) async fn submit(
    db: &DatabaseConnection,
    key: &SubmissionKey,
    payment: Payment,
    reserve: u64,
    instructions: PaymentInstructions,
) -> Result<SubmittedPayment, ApplicationError> {
    if key.wallet_id != payment.wallet_id {
        return Err(DataError::Validation("Submission wallet differs from payment wallet".into()).into());
    }
    let tx = db.begin().await.map_err(db_error)?;
    let id = Uuid::new_v4();
    let inserted = Entity::insert(ActiveModel {
        id: Set(id),
        wallet_id: Set(key.wallet_id),
        idempotency_key: Set(key.key),
        payment_id: Set(None),
        request_fingerprint: Set(key.fingerprint.clone()),
        instructions: Set(serde_json::to_value(instructions).expect("payment instructions serialize")),
        created_at: Set(Utc::now().naive_utc()),
        dispatched_at: Set(None),
    })
    .on_conflict(
        OnConflict::columns([Column::WalletId, Column::IdempotencyKey])
            .do_nothing()
            .to_owned(),
    )
    .try_insert()
    .exec(&tx)
    .await
    .map_err(db_error)?;
    if !matches!(inserted, TryInsertResult::Inserted(_)) {
        tx.rollback().await.map_err(db_error)?;
        return submitted(db, key)
            .await?
            .ok_or_else(|| DatabaseError::FindOne("Conflicting payment submission disappeared".into()).into());
    }
    let payment = reserve_in_transaction(&tx, payment, reserve).await?;
    Entity::update_many()
        .col_expr(Column::PaymentId, Expr::value(payment.id))
        .filter(Column::Id.eq(id))
        .exec(&tx)
        .await
        .map_err(db_error)?;
    append(&tx, &payment, ClientEventType::PaymentAccepted).await?;
    tx.commit().await.map_err(db_error)?;
    Ok(SubmittedPayment {
        payment,
        dispatched: false,
    })
}

pub(super) async fn claim(db: &DatabaseConnection) -> Result<Option<ClaimedPayment>, ApplicationError> {
    use super::models::payment;
    let candidate = Entity::find()
        .inner_join(payment::Entity)
        .filter(Column::DispatchedAt.is_null())
        .filter(payment::Column::Status.eq("Pending"))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .one(db)
        .await
        .map_err(db_error)?;
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    let payment_id = candidate.payment_id.expect("selected with an inner payment join");
    let tx = db.begin().await.map_err(db_error)?;
    let winner = Entity::update_many()
        .col_expr(Column::DispatchedAt, Expr::value(Utc::now().naive_utc()))
        .filter(Column::Id.eq(candidate.id))
        .filter(Column::DispatchedAt.is_null())
        .filter(Column::PaymentId.eq(payment_id))
        .exec(&tx)
        .await
        .map_err(db_error)?;
    if winner.rows_affected == 0 {
        return Ok(None);
    }
    let payments = SeaOrmPaymentRepository::new(&tx);
    // Serialize the processing event with terminal transitions from the node listener.
    use crate::domains::payment::PaymentStatus;
    if !payments
        .try_transition(payment_id, &[PaymentStatus::Pending], PaymentStatus::Pending)
        .await?
    {
        return Ok(None);
    }
    let payment = payments
        .find(payment_id)
        .await?
        .ok_or_else(|| DataError::NotFound("Payment disappeared before dispatch".into()))?;
    let instructions = serde_json::from_value(candidate.instructions)
        .map_err(|e| DatabaseError::FindOne(format!("Invalid stored payment instructions: {e}")))?;
    append(&tx, &payment, ClientEventType::PaymentProcessing).await?;
    tx.commit().await.map_err(db_error)?;
    Ok(Some(ClaimedPayment { payment, instructions }))
}

pub(super) async fn by_id(
    db: &DatabaseConnection,
    wallet_id: Option<Uuid>,
    id: Uuid,
) -> Result<Option<SubmittedPayment>, ApplicationError> {
    let mut query = Entity::find().filter(Column::PaymentId.eq(id));
    if let Some(owner_wallet) = wallet_id {
        query = query.filter(Column::WalletId.eq(owner_wallet));
    }
    let Some(row) = query.one(db).await.map_err(db_error)? else {
        return Ok(None);
    };
    let payment = SeaOrmPaymentRepository::new(db.clone()).find(id).await?;
    Ok(payment.map(|payment| SubmittedPayment {
        payment,
        dispatched: row.dispatched_at.is_some(),
    }))
}

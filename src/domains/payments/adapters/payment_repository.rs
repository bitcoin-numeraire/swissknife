use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::payments::entities::{Payment, PaymentFilter},
};

use super::{
    payment_model::{ActiveModel, Column, Entity},
    PaymentRepository,
};

#[derive(Clone)]
pub struct SeaOrmPaymentRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmPaymentRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PaymentRepository for SeaOrmPaymentRepository {
    async fn find(&self, id: Uuid) -> Result<Option<Payment>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_by_payment_hash(
        &self,
        payment_hash: &str,
    ) -> Result<Option<Payment>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::PaymentHash.eq(payment_hash))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(Into::into))
    }

    async fn find_many(&self, filter: PaymentFilter) -> Result<Vec<Payment>, DatabaseError> {
        let models = Entity::find()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.status, |q, s| {
                q.filter(Column::Status.eq(s.to_string()))
            })
            .apply_if(filter.ledger, |q, l| {
                q.filter(Column::Ledger.eq(l.to_string()))
            })
            .order_by(Column::CreatedAt, filter.order_direction.into())
            .offset(filter.offset)
            .limit(filter.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn insert(
        &self,
        txn: Option<&DatabaseTransaction>,
        payment: Payment,
    ) -> Result<Payment, DatabaseError> {
        let model = ActiveModel {
            user_id: Set(payment.user_id),
            ln_address: Set(payment.ln_address),
            amount_msat: Set(payment.amount_msat as i64),
            status: Set(payment.status.to_string()),
            ledger: Set(payment.ledger.to_string()),
            fee_msat: Set(payment.fee_msat.map(|v| v as i64)),
            payment_time: Set(payment.payment_time),
            payment_hash: Set(payment.payment_hash),
            description: Set(payment.description),
            metadata: Set(payment.metadata),
            ..Default::default()
        };

        let result = match txn {
            Some(txn) => model.insert(txn).await,
            None => model.insert(&self.db).await,
        };

        let model = result.map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn update(&self, payment: Payment) -> Result<Payment, DatabaseError> {
        let model = ActiveModel {
            id: Set(payment.id),
            status: Set(payment.status.to_string()),
            fee_msat: Set(payment.fee_msat.map(|v| v as i64)),
            payment_time: Set(payment.payment_time),
            payment_hash: Set(payment.payment_hash),
            payment_preimage: Set(payment.payment_preimage),
            error: Set(payment.error),
            amount_msat: Set(payment.amount_msat as i64),
            metadata: Set(payment.metadata),
            success_action: Set(payment
                .success_action
                .and_then(|action| serde_json::to_value(action).ok())),
            ..Default::default()
        };

        let model = model
            .update(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(model.into())
    }

    async fn delete_many(&self, filter: PaymentFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.status, |q, s| {
                q.filter(Column::Status.eq(s.to_string()))
            })
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

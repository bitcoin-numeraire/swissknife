use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Set, Unchanged,
};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::payment::{Payment, PaymentFilter, PaymentRepository},
    infra::database::sea_orm::models::{
        btc_output,
        payment::{ActiveModel, Column, Entity, Model as PaymentModel},
    },
};

#[derive(Clone)]
pub struct SeaOrmPaymentRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmPaymentRepository {
    fn map_with_output(model: PaymentModel, btc_output: Option<btc_output::Model>) -> Payment {
        let mut payment: Payment = model.into();

        if let Some(output) = btc_output {
            let bitcoin = payment.bitcoin.get_or_insert_with(Default::default);
            bitcoin.btc_output_id = Some(output.id);
            bitcoin.btc_output = Some(output.into());
        }

        payment
    }

    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PaymentRepository for SeaOrmPaymentRepository {
    async fn find(&self, id: Uuid) -> Result<Option<Payment>, DatabaseError> {
        let model = Entity::find_by_id(id)
            .find_also_related(btc_output::Entity)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(|(payment, btc_output)| Self::map_with_output(payment, btc_output)))
    }

    async fn find_by_payment_hash(&self, payment_hash: &str) -> Result<Option<Payment>, DatabaseError> {
        let model = Entity::find()
            .filter(Column::PaymentHash.eq(payment_hash))
            .find_also_related(btc_output::Entity)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        Ok(model.map(|(payment, btc_output)| Self::map_with_output(payment, btc_output)))
    }

    async fn find_many(&self, filter: PaymentFilter) -> Result<Vec<Payment>, DatabaseError> {
        let models = Entity::find()
            .apply_if(filter.wallet_id, |q, wallet| q.filter(Column::WalletId.eq(wallet)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.status, |q, s| q.filter(Column::Status.eq(s.to_string())))
            .apply_if(filter.ledger, |q, l| q.filter(Column::Ledger.eq(l.to_string())))
            .apply_if(filter.ln_addresses, |q, ln_addresses| {
                q.filter(Column::LnAddress.is_in(ln_addresses))
            })
            .apply_if(filter.btc_addresses, |q, btc_addresses| {
                q.filter(Column::BtcAddress.is_in(btc_addresses))
            })
            .order_by(Column::CreatedAt, filter.order_direction.into())
            .offset(filter.offset)
            .limit(filter.limit)
            .find_also_related(btc_output::Entity)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models
            .into_iter()
            .map(|(payment, btc_output)| Self::map_with_output(payment, btc_output))
            .collect())
    }

    async fn insert(&self, txn: Option<&DatabaseTransaction>, payment: Payment) -> Result<Payment, DatabaseError> {
        let (ln_address, payment_hash, payment_preimage, metadata, success_action) = payment
            .lightning
            .as_ref()
            .map(|lightning| {
                (
                    lightning.ln_address.clone(),
                    lightning.payment_hash.clone(),
                    lightning.payment_preimage.clone(),
                    lightning.metadata.clone(),
                    lightning
                        .success_action
                        .clone()
                        .and_then(|action| serde_json::to_value(action).ok()),
                )
            })
            .unwrap_or_default();

        let (btc_address, btc_txid, btc_output_id) = payment
            .bitcoin
            .as_ref()
            .map(|bitcoin| {
                (
                    bitcoin.destination_address.clone(),
                    bitcoin.txid.clone(),
                    bitcoin.btc_output_id,
                )
            })
            .unwrap_or_default();

        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            wallet_id: Set(payment.wallet_id),
            ln_address: Set(ln_address),
            btc_address: Set(btc_address),
            amount_msat: Set(payment.amount_msat as i64),
            status: Set(payment.status.to_string()),
            ledger: Set(payment.ledger.to_string()),
            currency: Set(payment.currency.to_string()),
            fee_msat: Set(payment.fee_msat.map(|v| v as i64)),
            payment_time: Set(payment.payment_time.map(|t| t.naive_utc())),
            payment_hash: Set(payment_hash.or(btc_txid)),
            description: Set(payment.description),
            metadata: Set(metadata),
            success_action: Set(success_action),
            payment_preimage: Set(payment_preimage),
            btc_output_id: Set(btc_output_id),
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
        let (ln_address, payment_hash, payment_preimage, metadata, success_action) = payment
            .lightning
            .as_ref()
            .map(|lightning| {
                (
                    lightning.ln_address.clone(),
                    lightning.payment_hash.clone(),
                    lightning.payment_preimage.clone(),
                    lightning.metadata.clone(),
                    lightning
                        .success_action
                        .clone()
                        .and_then(|action| serde_json::to_value(action).ok()),
                )
            })
            .unwrap_or_default();

        let (btc_address, btc_txid, btc_output_id) = payment
            .bitcoin
            .as_ref()
            .map(|bitcoin| {
                (
                    bitcoin.destination_address.clone(),
                    bitcoin.txid.clone(),
                    bitcoin.btc_output_id,
                )
            })
            .unwrap_or_default();

        let model = ActiveModel {
            id: Unchanged(payment.id),
            status: Set(payment.status.to_string()),
            fee_msat: Set(payment.fee_msat.map(|v| v as i64)),
            payment_time: Set(payment.payment_time.map(|t| t.naive_utc())),
            payment_hash: Set(payment_hash.or(btc_txid)),
            payment_preimage: Set(payment_preimage),
            error: Set(payment.error),
            amount_msat: Set(payment.amount_msat as i64),
            metadata: Set(metadata),
            ln_address: Set(ln_address),
            btc_address: Set(btc_address),
            btc_output_id: Set(btc_output_id),
            success_action: Set(success_action),
            updated_at: Set(Some(Utc::now().naive_utc())),
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
            .apply_if(filter.wallet_id, |q, wallet| q.filter(Column::WalletId.eq(wallet)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .apply_if(filter.status, |q, s| q.filter(Column::Status.eq(s.to_string())))
            .apply_if(filter.ln_addresses, |q, ln_addresses| {
                q.filter(Column::LnAddress.is_in(ln_addresses))
            })
            .apply_if(filter.btc_addresses, |q, btc_addresses| {
                q.filter(Column::BtcAddress.is_in(btc_addresses))
            })
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

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
        ln_address::Model as LnAddressModel,
        payment::Column as PaymentColumn,
        prelude::{Invoice, LnAddress, Payment},
        wallet::{ActiveModel, Column, Entity},
        wallet_overview::WalletOverviewModel,
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
        let wallet_q = Entity::find()
            .left_join(Payment)
            .left_join(Invoice)
            .column_as(InvoiceColumn::AmountReceivedMsat.sum(), "received_msat")
            .column_as(PaymentColumn::AmountMsat.sum(), "sent_msat")
            .column_as(PaymentColumn::FeeMsat.sum(), "fees_paid_msat")
            .column_as(PaymentColumn::Id.count(), "n_payments")
            .column_as(InvoiceColumn::Id.count(), "n_invoices")
            .column_as(Expr::col(PaymentColumn::LnAddress).count_distinct(), "n_contacts")
            .group_by(Column::Id)
            .group_by(Column::UserId)
            .group_by(Column::CreatedAt)
            .group_by(Column::UpdatedAt)
            .find_also_related(LnAddress);

        let results = wallet_q
            .into_model::<WalletOverviewModel, LnAddressModel>()
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|(wallet_model, ln_address_model)| {
                let mut overview: WalletOverview = wallet_model.into();
                overview.ln_address = ln_address_model.map(Into::into);
                overview
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
            .column_as(InvoiceColumn::AmountReceivedMsat.sum(), "received_msat")
            .into_tuple::<Option<i64>>();

        let sent = Payment::find()
            .filter(PaymentColumn::WalletId.eq(id))
            .filter(
                PaymentColumn::Status.is_in([PaymentStatus::Settled.to_string(), PaymentStatus::Pending.to_string()]),
            )
            .select_only()
            .column_as(PaymentColumn::AmountMsat.sum(), "sent_msat")
            .column_as(PaymentColumn::FeeMsat.sum(), "fees_paid_msat")
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

        Ok(Balance::new(
            received.unwrap_or_default(),
            sent_msat.unwrap_or_default(),
            fees_paid_msat.unwrap_or_default(),
        ))
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

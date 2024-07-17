use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    DatabaseTransaction, EntityTrait, FromQueryResult, ModelTrait, QueryFilter, QueryOrder,
    QuerySelect, Statement,
};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::{
        payment::PaymentStatus,
        wallet::{Balance, Contact, Wallet, WalletRepository},
    },
    infra::database::sea_orm::models::{
        balance::BalanceModel,
        contact::ContactModel,
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
                wallet.balance = balance.into();
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

    async fn find_many(&self) -> Result<Vec<Wallet>, DatabaseError> {
        let models = Entity::find()
            .find_also_related(LnAddress)
            .order_by_desc(Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        Ok(models
            .into_iter()
            .map(|(wallet_model, ln_address_model)| {
                let mut wallet: Wallet = wallet_model.into();
                wallet.ln_address = ln_address_model.map(Into::into);
                wallet
            })
            .collect())
    }

    async fn insert(&self, user_id: &str) -> Result<Wallet, DatabaseError> {
        let model = ActiveModel {
            user_id: Set(user_id.to_string()),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }

    async fn get_balance(
        &self,
        txn: Option<&DatabaseTransaction>,
        id: Uuid,
    ) -> Result<Balance, DatabaseError> {
        let query = BalanceModel::find_by_statement(Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            WITH sent AS (
                SELECT
                    SUM(amount_msat) FILTER (WHERE status IN ('Settled', 'Pending')) AS sent_msat,
                    SUM(COALESCE(fee_msat, 0)) FILTER (WHERE status = 'Settled') AS fees_paid_msat
                FROM payment
                WHERE wallet_id = $1
            ),
            received AS (
                SELECT SUM(amount_received_msat) AS received_msat
                FROM invoice
                WHERE wallet_id = $1
            )
            SELECT
                COALESCE(received.received_msat, 0)::BIGINT AS received_msat,
                COALESCE(sent.sent_msat, 0)::BIGINT AS sent_msat,
                COALESCE(sent.fees_paid_msat, 0)::BIGINT AS fees_paid_msat
            FROM received, sent;
            "#,
            [id.into()],
        ));

        let result = match txn {
            Some(txn) => query.one(txn).await,
            None => query.one(&self.db).await,
        };

        let result = result.map_err(|e| DatabaseError::FindByStatement(e.to_string()))?;

        match result {
            Some(model) => Ok(model.into()),
            None => Ok(Balance::default()),
        }
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
}

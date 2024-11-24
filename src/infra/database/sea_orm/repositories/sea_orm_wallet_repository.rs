use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    DatabaseTransaction, EntityTrait, FromQueryResult, ModelTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, Statement,
};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::{
        payment::PaymentStatus,
        wallet::{Balance, Contact, Wallet, WalletFilter, WalletOverview, WalletRepository},
    },
    infra::database::sea_orm::models::{
        balance::BalanceModel,
        contact::ContactModel,
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
        let models = WalletOverviewModel::find_by_statement(Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
                WITH wallet_balances AS (
                    WITH sent AS (
                        SELECT
                            wallet_id,
                            SUM(amount_msat) FILTER (WHERE status IN ('Settled', 'Pending')) AS sent_msat,
                            SUM(COALESCE(fee_msat, 0)) FILTER (WHERE status = 'Settled') AS fees_paid_msat
                        FROM payment
                        GROUP BY wallet_id
                    ),
                    received AS (
                        SELECT
                            wallet_id,
                            SUM(amount_received_msat) AS received_msat
                        FROM invoice
                        GROUP BY wallet_id
                    )
                    SELECT
                        w.id AS wallet_id,
                        COALESCE(received.received_msat, 0)::BIGINT AS received_msat,
                        COALESCE(sent.sent_msat, 0)::BIGINT AS sent_msat,
                        COALESCE(sent.fees_paid_msat, 0)::BIGINT AS fees_paid_msat
                    FROM wallet w
                    LEFT JOIN received ON w.id = received.wallet_id
                    LEFT JOIN sent ON w.id = sent.wallet_id
                ),
                payment_counts AS (
                    SELECT wallet_id, COUNT(*) AS n_payments
                    FROM payment
                    GROUP BY wallet_id
                ),
                invoice_counts AS (
                    SELECT wallet_id, COUNT(*) AS n_invoices
                    FROM invoice
                    GROUP BY wallet_id
                ),
                contact_counts AS (
                    SELECT wallet_id, COUNT(DISTINCT ln_address) AS n_contacts
                    FROM payment
                    WHERE ln_address IS NOT NULL AND status = 'Settled'
                    GROUP BY wallet_id
                )
                SELECT
                    w.id,
                    w.user_id,
                    w.created_at,
                    w.updated_at,
                    COALESCE(wb.received_msat, 0)::BIGINT AS received_msat,
                    COALESCE(wb.sent_msat, 0)::BIGINT AS sent_msat,
                    COALESCE(wb.fees_paid_msat, 0)::BIGINT AS fees_paid_msat,
                    COALESCE(pc.n_payments, 0) AS n_payments,
                    COALESCE(ic.n_invoices, 0) AS n_invoices,
                    COALESCE(cc.n_contacts, 0) AS n_contacts,
                    la.id AS ln_address_id,
                    la.username AS ln_address_username
                FROM wallet w
                LEFT JOIN wallet_balances wb ON w.id = wb.wallet_id
                LEFT JOIN payment_counts pc ON w.id = pc.wallet_id
                LEFT JOIN invoice_counts ic ON w.id = ic.wallet_id
                LEFT JOIN contact_counts cc ON w.id = cc.wallet_id
                LEFT JOIN ln_address la ON w.id = la.wallet_id
                ORDER BY w.created_at DESC
                "#,
            [],
        )).all(&self.db).await.map_err(|e| DatabaseError::FindByStatement(e.to_string()))?;

        Ok(models.into_iter().map(Into::into).collect())
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

    async fn delete_many(&self, filter: WalletFilter) -> Result<u64, DatabaseError> {
        let result = Entity::delete_many()
            .apply_if(filter.user_id, |q, user| q.filter(Column::UserId.eq(user)))
            .apply_if(filter.ids, |q, ids| q.filter(Column::Id.is_in(ids)))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }

    async fn count(&self) -> Result<u64, DatabaseError> {
        let count = Entity::find()
            .count(&self.db)
            .await
            .map_err(|e| DatabaseError::Count(e.to_string()))?;

        Ok(count)
    }
}

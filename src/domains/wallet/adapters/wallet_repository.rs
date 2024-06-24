use async_trait::async_trait;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, FromQueryResult, Statement,
};

use crate::application::errors::DatabaseError;
use crate::domains::wallet::entities::UserBalance;

use super::user_balance_model::UserBalanceModel;
use super::WalletRepository;

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
    async fn get_balance(
        &self,
        txn: Option<&DatabaseTransaction>,
        user: &str,
    ) -> Result<UserBalance, DatabaseError> {
        let query = UserBalanceModel::find_by_statement(Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            WITH sent AS (
                SELECT
                    SUM(amount_msat) FILTER (WHERE status IN ('SETTLED', 'PENDING')) AS sent_msat,
                    SUM(COALESCE(fee_msat, 0)) FILTER (WHERE status = 'SETTLED') AS fees_paid_msat
                FROM payment
                WHERE user_id = $1
            ),
            received AS (
                SELECT SUM(amount_msat) AS received_msat
                FROM invoice
                WHERE user_id = $1 AND payment_time IS NOT NULL
            )
            SELECT
                COALESCE(received.received_msat, 0)::BIGINT AS received_msat,
                COALESCE(sent.sent_msat, 0)::BIGINT AS sent_msat,
                COALESCE(sent.fees_paid_msat, 0)::BIGINT AS fees_paid_msat
            FROM received, sent;
            "#,
            [user.into()],
        ));

        let result = match txn {
            Some(txn) => query.one(txn).await,
            None => query.one(&self.db).await,
        };

        let result = result.map_err(|e| DatabaseError::FindByStatement(e.to_string()))?;

        match result {
            Some(model) => Ok(model.into()),
            None => Ok(UserBalance::default()),
        }
    }
}

use crate::{
    application::errors::DatabaseError,
    domains::user::{Account, AccountRepository},
    infra::database::sea_orm::models::{
        account::{ActiveModel, Column, Entity},
        ln_address::Column as LnAddressColumn,
        prelude::{LnAddress, Wallet},
    },
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

#[derive(Clone)]
pub struct SeaOrmAccountRepository {
    pub db: DatabaseConnection,
}

impl SeaOrmAccountRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AccountRepository for SeaOrmAccountRepository {
    async fn find_by_sub(&self, sub: &str) -> Result<Option<Account>, DatabaseError> {
        let model = Entity::find()
            .find_also_related(Wallet)
            .filter(Column::Sub.eq(sub))
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        match model {
            Some((account_model, wallet_model)) => {
                let ln_address_model = LnAddress::find()
                    .filter(LnAddressColumn::UserId.eq(account_model.id))
                    .one(&self.db)
                    .await
                    .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

                let mut user: Account = account_model.into();
                user.wallet = wallet_model
                    .expect("Wallet must exist if account exists")
                    .into();
                user.ln_address = ln_address_model.map(Into::into);

                Ok(Some(user))
            }
            None => Ok(None),
        }
    }

    async fn insert(&self, sub: &str) -> Result<Account, DatabaseError> {
        let model = ActiveModel {
            sub: Set(sub.to_string()),
            ..Default::default()
        };

        let model = model
            .insert(&self.db)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(model.into())
    }
}

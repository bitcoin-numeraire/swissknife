use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::application::{
    composition::AppStore,
    errors::{ApplicationError, DataError},
};

use super::{Account, AccountFilter, AccountPreferences, AccountUseCases, CreateAccountRequest, Permission};

pub struct AccountService {
    store: AppStore,
}

impl AccountService {
    pub fn new(store: AppStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl AccountUseCases for AccountService {
    async fn create(&self, request: CreateAccountRequest) -> Result<Account, ApplicationError> {
        debug!("Creating account");

        let account = self
            .store
            .account
            .insert(request.display_name, &request.permissions)
            .await?;

        info!(id = %account.id, "Account created successfully");
        Ok(account)
    }

    async fn get(&self, id: Uuid) -> Result<Account, ApplicationError> {
        trace!(%id, "Fetching account");

        let account = self
            .store
            .account
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Account not found.".to_string()))?;

        debug!(%id, "Account fetched successfully");
        Ok(account)
    }

    async fn list(&self, filter: AccountFilter) -> Result<Vec<Account>, ApplicationError> {
        trace!(?filter, "Listing accounts");

        let accounts = self.store.account.find_many(filter.clone()).await?;

        debug!(?filter, "Accounts listed successfully");
        Ok(accounts)
    }

    async fn update(&self, id: Uuid, display_name: Option<String>) -> Result<Account, ApplicationError> {
        debug!(%id, "Updating account");

        let mut account = self
            .store
            .account
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Account not found.".to_string()))?;
        account.display_name = display_name;
        let account = self.store.account.update(account).await?;

        info!(%id, "Account updated successfully");
        Ok(account)
    }

    async fn update_permissions(&self, id: Uuid, permissions: Vec<Permission>) -> Result<Account, ApplicationError> {
        debug!(%id, "Updating account permissions");

        let mut account = self
            .store
            .account
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Account not found.".to_string()))?;
        let mut unique_permissions = Vec::new();
        for permission in permissions {
            if !unique_permissions.contains(&permission) {
                unique_permissions.push(permission);
            }
        }
        account.permissions = Some(unique_permissions);
        let account = self.store.account.update(account).await?;

        info!(%id, "Account permissions updated successfully");
        Ok(account)
    }

    async fn update_preferences(
        &self,
        id: Uuid,
        dashboard_settings: Value,
    ) -> Result<AccountPreferences, ApplicationError> {
        debug!(%id, "Updating account preferences");

        let preferences = self
            .store
            .account
            .update_preferences(id, dashboard_settings)
            .await?
            .ok_or_else(|| DataError::NotFound("Account preferences not found.".to_string()))?;

        info!(%id, "Account preferences updated successfully");
        Ok(preferences)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting account");

        let n_deleted = self
            .store
            .account
            .delete_many(AccountFilter {
                ids: Some(vec![id]),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Account not found.".to_string()).into());
        }

        info!(%id, "Account deleted successfully");
        Ok(())
    }

    async fn delete_many(&self, filter: AccountFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting accounts");

        let n_deleted = self.store.account.delete_many(filter.clone()).await?;

        info!(?filter, n_deleted, "Accounts deleted successfully");
        Ok(n_deleted)
    }
}

#[cfg(test)]
mod tests {
    use crate::application::composition::MockAppStoreBuilder;

    use super::*;

    fn account_fixture(id: Uuid) -> Account {
        Account {
            id,
            ..Default::default()
        }
    }

    mod create {
        use super::*;

        #[tokio::test]
        async fn creates_an_account_without_an_identity() {
            let account_id = Uuid::new_v4();
            let mut store = MockAppStoreBuilder::new();
            store
                .account
                .expect_insert()
                .withf(|display_name, permissions| {
                    display_name.as_deref() == Some("Operator") && permissions == [Permission::ReadWallet]
                })
                .times(1)
                .returning(move |_, _| Ok(account_fixture(account_id)));
            let service = AccountService::new(store.build());

            let account = service
                .create(CreateAccountRequest {
                    display_name: Some("Operator".to_string()),
                    permissions: vec![Permission::ReadWallet],
                })
                .await
                .unwrap();

            assert_eq!(account.id, account_id);
            assert!(account.identity.is_none());
        }
    }

    mod list {
        use super::*;

        #[tokio::test]
        async fn returns_repository_accounts() {
            let account_id = Uuid::new_v4();
            let mut store = MockAppStoreBuilder::new();
            store
                .account
                .expect_find_many()
                .withf(|filter| filter.limit == Some(10))
                .times(1)
                .returning(move |_| Ok(vec![account_fixture(account_id)]));
            let service = AccountService::new(store.build());

            let accounts = service
                .list(AccountFilter {
                    limit: Some(10),
                    ..Default::default()
                })
                .await
                .unwrap();

            assert_eq!(accounts.len(), 1);
            assert_eq!(accounts[0].id, account_id);
        }
    }

    mod update {
        use super::*;

        #[tokio::test]
        async fn reports_a_missing_account() {
            let mut store = MockAppStoreBuilder::new();
            store.account.expect_find().times(1).returning(|_| Ok(None));
            let service = AccountService::new(store.build());

            let error = service.update(Uuid::new_v4(), None).await.unwrap_err();

            assert!(matches!(error, ApplicationError::Data(DataError::NotFound(_))));
        }
    }

    mod update_permissions {
        use super::*;

        #[tokio::test]
        async fn replaces_account_permissions() {
            let account_id = Uuid::new_v4();
            let mut updated = account_fixture(account_id);
            updated.permissions = Some(vec![Permission::ReadAccount]);
            let mut store = MockAppStoreBuilder::new();
            store
                .account
                .expect_find()
                .withf(move |id| *id == account_id)
                .times(1)
                .returning(move |_| Ok(Some(account_fixture(account_id))));
            store
                .account
                .expect_update()
                .withf(move |account| {
                    account.id == account_id && account.permissions == Some(vec![Permission::ReadAccount])
                })
                .times(1)
                .returning(move |_| Ok(updated.clone()));
            let service = AccountService::new(store.build());

            let account = service
                .update_permissions(account_id, vec![Permission::ReadAccount, Permission::ReadAccount])
                .await
                .unwrap();

            assert_eq!(account.permissions, Some(vec![Permission::ReadAccount]));
        }
    }

    mod delete {
        use super::*;

        #[tokio::test]
        async fn reports_a_missing_account() {
            let mut store = MockAppStoreBuilder::new();
            store
                .account
                .expect_delete_many()
                .withf(|filter| filter.ids.as_ref().is_some_and(|ids| ids.len() == 1))
                .times(1)
                .returning(|_| Ok(0));
            let service = AccountService::new(store.build());

            let error = service.delete(Uuid::new_v4()).await.unwrap_err();

            assert!(matches!(error, ApplicationError::Data(DataError::NotFound(_))));
        }
    }

    mod delete_many {
        use super::*;

        #[tokio::test]
        async fn returns_the_number_of_deleted_accounts() {
            let mut store = MockAppStoreBuilder::new();
            store.account.expect_delete_many().times(1).returning(|_| Ok(3));
            let service = AccountService::new(store.build());

            let deleted = service.delete_many(AccountFilter::default()).await.unwrap();

            assert_eq!(deleted, 3);
        }
    }
}

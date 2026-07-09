use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::application::{
    composition::AppStore,
    errors::{ApplicationError, DataError},
};

use super::{
    Account, AccountFilter, AccountPreferences, AccountUseCases, AuthProvider, CreateAccountRequest, Permission,
    UpdateAccountRequest,
};

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
        debug!(provider = %request.provider, subject = %request.subject, "Creating account");

        if request.provider == AuthProvider::OAuth2 && !request.permissions.is_empty() {
            return Err(
                DataError::Validation("OAuth2 account permissions are provided by token claims.".to_string()).into(),
            );
        }

        let account = self
            .store
            .account
            .upsert(
                request.provider,
                &request.subject,
                request.display_name,
                &request.permissions,
            )
            .await?;

        info!(id = %account.id, "Account created or already existed");
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

    async fn update(&self, id: Uuid, request: UpdateAccountRequest) -> Result<Account, ApplicationError> {
        debug!(%id, "Updating account");

        let account = self
            .store
            .account
            .update(id, request.display_name)
            .await?
            .ok_or_else(|| DataError::NotFound("Account not found.".to_string()))?;

        info!(%id, "Account updated successfully");
        Ok(account)
    }

    async fn update_permissions(&self, id: Uuid, permissions: Vec<Permission>) -> Result<Account, ApplicationError> {
        debug!(%id, "Updating account permissions");

        let account = self.get(id).await?;
        if account.identity.as_ref().map(|identity| identity.provider) == Some(AuthProvider::OAuth2) {
            return Err(
                DataError::Validation("OAuth2 account permissions are provided by token claims.".to_string()).into(),
            );
        }

        let account = self
            .store
            .account
            .update_permissions(id, &permissions)
            .await?
            .ok_or_else(|| DataError::NotFound("Account not found.".to_string()))?;

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

        if !self.store.account.delete(id).await? {
            return Err(DataError::NotFound("Account not found.".to_string()).into());
        }

        info!(%id, "Account deleted successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::application::composition::MockAppStoreBuilder;

    use super::*;
    use crate::domains::user::AuthIdentity;

    fn account_fixture(id: Uuid, provider: AuthProvider) -> Account {
        Account {
            id,
            identity: Some(AuthIdentity {
                provider,
                subject: "subject".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    mod create {
        use super::*;

        #[tokio::test]
        async fn delegates_local_account_creation_to_the_repository() {
            let account_id = Uuid::new_v4();
            let mut store = MockAppStoreBuilder::new();
            store
                .account
                .expect_upsert()
                .withf(|provider, subject, display_name, permissions| {
                    *provider == AuthProvider::Jwt
                        && subject == "operator"
                        && display_name.as_deref() == Some("Operator")
                        && permissions == [Permission::ReadWallet]
                })
                .times(1)
                .returning(move |_, _, _, _| Ok(account_fixture(account_id, AuthProvider::Jwt)));
            let service = AccountService::new(store.build());

            let account = service
                .create(CreateAccountRequest {
                    display_name: Some("Operator".to_string()),
                    provider: AuthProvider::Jwt,
                    subject: "operator".to_string(),
                    permissions: vec![Permission::ReadWallet],
                })
                .await
                .unwrap();

            assert_eq!(account.id, account_id);
        }

        #[tokio::test]
        async fn rejects_database_permissions_for_oauth2_accounts() {
            let service = AccountService::new(MockAppStoreBuilder::new().build());

            let error = service
                .create(CreateAccountRequest {
                    display_name: None,
                    provider: AuthProvider::OAuth2,
                    subject: "oauth-subject".to_string(),
                    permissions: vec![Permission::ReadWallet],
                })
                .await
                .unwrap_err();

            assert!(matches!(error, ApplicationError::Data(DataError::Validation(_))));
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
                .returning(move |_| Ok(vec![account_fixture(account_id, AuthProvider::Jwt)]));
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
            store.account.expect_update().times(1).returning(|_, _| Ok(None));
            let service = AccountService::new(store.build());

            let error = service
                .update(Uuid::new_v4(), UpdateAccountRequest { display_name: None })
                .await
                .unwrap_err();

            assert!(matches!(error, ApplicationError::Data(DataError::NotFound(_))));
        }
    }

    mod update_permissions {
        use super::*;

        #[tokio::test]
        async fn rejects_oauth2_accounts() {
            let account_id = Uuid::new_v4();
            let mut store = MockAppStoreBuilder::new();
            store
                .account
                .expect_find()
                .withf(move |id| *id == account_id)
                .times(1)
                .returning(move |_| Ok(Some(account_fixture(account_id, AuthProvider::OAuth2))));
            let service = AccountService::new(store.build());

            let error = service
                .update_permissions(account_id, vec![Permission::ReadWallet])
                .await
                .unwrap_err();

            assert!(matches!(error, ApplicationError::Data(DataError::Validation(_))));
        }

        #[tokio::test]
        async fn replaces_local_account_permissions() {
            let account_id = Uuid::new_v4();
            let mut updated = account_fixture(account_id, AuthProvider::Jwt);
            updated.permissions = Some(vec![Permission::ReadAccount]);
            let mut store = MockAppStoreBuilder::new();
            store
                .account
                .expect_find()
                .times(1)
                .returning(move |_| Ok(Some(account_fixture(account_id, AuthProvider::Jwt))));
            store
                .account
                .expect_update_permissions()
                .withf(move |id, permissions| *id == account_id && permissions == [Permission::ReadAccount])
                .times(1)
                .returning(move |_, _| Ok(Some(updated.clone())));
            let service = AccountService::new(store.build());

            let account = service
                .update_permissions(account_id, vec![Permission::ReadAccount])
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
            store.account.expect_delete().times(1).returning(|_| Ok(false));
            let service = AccountService::new(store.build());

            let error = service.delete(Uuid::new_v4()).await.unwrap_err();

            assert!(matches!(error, ApplicationError::Data(DataError::NotFound(_))));
        }
    }
}

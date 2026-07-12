use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use swissknife_types::CreateApiKeyRequest;

use crate::application::errors::ApplicationError;

use super::{Account, AccountFilter, AccountPreferences, ApiKey, ApiKeyFilter, CreateAccountRequest, Permission, User};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AuthUseCases: Send + Sync {
    async fn sign_up(&self, password: String) -> Result<String, ApplicationError>;
    async fn sign_in(&self, password: String) -> Result<String, ApplicationError>;
    async fn change_password(&self, current_password: String, new_password: String) -> Result<(), ApplicationError>;
    async fn authenticate_jwt(&self, token: &str) -> Result<User, ApplicationError>;
    async fn authenticate_api_key(&self, token: Vec<u8>) -> Result<User, ApplicationError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AccountUseCases: Send + Sync {
    async fn create(&self, request: CreateAccountRequest) -> Result<Account, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<Account, ApplicationError>;
    async fn list(&self, filter: AccountFilter) -> Result<Vec<Account>, ApplicationError>;
    async fn update(&self, id: Uuid, display_name: Option<String>) -> Result<Account, ApplicationError>;
    async fn update_permissions(&self, id: Uuid, permissions: Vec<Permission>) -> Result<Account, ApplicationError>;
    async fn update_preferences(
        &self,
        id: Uuid,
        dashboard_settings: Value,
    ) -> Result<AccountPreferences, ApplicationError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn delete_many(&self, filter: AccountFilter) -> Result<u64, ApplicationError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ApiKeyUseCases: Send + Sync {
    async fn generate(&self, user: User, request: CreateApiKeyRequest) -> Result<ApiKey, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<ApiKey, ApplicationError>;
    async fn list(&self, filter: ApiKeyFilter) -> Result<Vec<ApiKey>, ApplicationError>;
    async fn revoke(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn revoke_many(&self, filter: ApiKeyFilter) -> Result<u64, ApplicationError>;
}

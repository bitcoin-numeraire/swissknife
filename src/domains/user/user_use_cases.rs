use async_trait::async_trait;
use uuid::Uuid;

use crate::application::{dtos::CreateApiKeyRequest, errors::ApplicationError};

use super::{ApiKey, ApiKeyFilter, User};

#[async_trait]
pub trait AuthUseCases: Send + Sync {
    async fn sign_up(&self, password: String) -> Result<String, ApplicationError>;
    async fn sign_in(&self, password: String) -> Result<String, ApplicationError>;
    async fn authenticate_jwt(&self, token: &str) -> Result<User, ApplicationError>;
    async fn authenticate_api_key(&self, token: Vec<u8>) -> Result<User, ApplicationError>;
}

#[async_trait]
pub trait ApiKeyUseCases: Send + Sync {
    async fn generate(&self, user: User, request: CreateApiKeyRequest) -> Result<ApiKey, ApplicationError>;
    async fn get(&self, id: Uuid) -> Result<ApiKey, ApplicationError>;
    async fn list(&self, filter: ApiKeyFilter) -> Result<Vec<ApiKey>, ApplicationError>;
    async fn revoke(&self, id: Uuid) -> Result<(), ApplicationError>;
    async fn revoke_many(&self, filter: ApiKeyFilter) -> Result<u64, ApplicationError>;
}

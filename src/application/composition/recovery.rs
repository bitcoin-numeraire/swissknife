use std::sync::Arc;

use crate::{
    application::errors::ApplicationError,
    domains::account::{LocalCredentialRepository, LoginRecoveryService, LoginRecoveryUseCases},
    infra::database::sea_orm::SeaOrmStore,
};

use super::{AppConfig, AuthProvider};

/// Recovery only needs the database; unavailable Lightning/OIDC services must not block it.
pub struct RecoveryAdapters {
    pub local_credential: Arc<dyn LocalCredentialRepository>,
}

impl RecoveryAdapters {
    pub async fn new(config: &AppConfig) -> Result<Self, ApplicationError> {
        let mut database = config.database.clone();
        database.sqlx_logging = Some(false);
        let store = SeaOrmStore::connect(database).await?;
        Ok(Self {
            local_credential: store.local_credential,
        })
    }
}

pub struct RecoveryServices {
    pub login: Box<dyn LoginRecoveryUseCases>,
}

impl RecoveryServices {
    pub fn new(provider: AuthProvider, adapters: RecoveryAdapters) -> Self {
        Self {
            login: Box::new(LoginRecoveryService::new(adapters.local_credential, provider)),
        }
    }
}

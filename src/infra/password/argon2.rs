use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, SaltString},
    Argon2, PasswordHasher as _, PasswordVerifier,
};
use async_trait::async_trait;
use tokio::sync::{OnceCell, Semaphore};

use crate::{
    application::errors::{ApplicationError, AuthenticationError},
    domains::account::PasswordHasher,
};

pub struct Argon2PasswordHasher {
    workers: Arc<Semaphore>,
    admitted: Arc<Semaphore>,
    dummy_hash: OnceCell<String>,
}

impl Argon2PasswordHasher {
    pub fn new() -> Self {
        Self {
            workers: Arc::new(Semaphore::new(4)),
            admitted: Arc::new(Semaphore::new(32)),
            dummy_hash: OnceCell::new(),
        }
    }
}

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    async fn hash(&self, password: String) -> Result<String, ApplicationError> {
        let admitted = self
            .admitted
            .clone()
            .try_acquire_owned()
            .map_err(|_| AuthenticationError::RateLimited)?;
        let permit = self
            .workers
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AuthenticationError::RateLimited)?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let _admitted = admitted;
            Argon2::default()
                .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
                .map(|hash| hash.to_string())
                .map_err(|_| AuthenticationError::Hash("Password hashing failed".into()))
        })
        .await
        .map_err(|_| AuthenticationError::Hash("Password worker failed".into()))?
        .map_err(Into::into)
    }

    async fn verify(&self, password: String, stored: Option<String>) -> Result<bool, ApplicationError> {
        if password.len() > 1024 {
            return Ok(false);
        }
        let real = stored.is_some();
        let stored = match stored {
            Some(stored) => stored,
            None => self
                .dummy_hash
                .get_or_try_init(|| self.hash(uuid::Uuid::new_v4().to_string()))
                .await?
                .clone(),
        };
        let admitted = self
            .admitted
            .clone()
            .try_acquire_owned()
            .map_err(|_| AuthenticationError::RateLimited)?;
        let permit = self
            .workers
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AuthenticationError::RateLimited)?;
        let valid = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let _admitted = admitted;
            PasswordHash::new(&stored)
                .is_ok_and(|hash| Argon2::default().verify_password(password.as_bytes(), &hash).is_ok())
        })
        .await
        .map_err(|_| AuthenticationError::Hash("Password worker failed".into()))?;
        Ok(real && valid)
    }
}

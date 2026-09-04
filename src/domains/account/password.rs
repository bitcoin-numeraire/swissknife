use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, SaltString},
    Argon2, PasswordHasher, PasswordVerifier,
};
use tokio::sync::{OnceCell, Semaphore};

use crate::application::errors::{ApplicationError, AuthenticationError, DataError};

pub struct PasswordService {
    workers: Arc<Semaphore>,
    admitted: Arc<Semaphore>,
    dummy_hash: OnceCell<String>,
}

impl PasswordService {
    pub fn new() -> Self {
        Self {
            workers: Arc::new(Semaphore::new(4)),
            admitted: Arc::new(Semaphore::new(32)),
            dummy_hash: OnceCell::new(),
        }
    }

    pub fn validate(password: &str) -> Result<(), ApplicationError> {
        if password.chars().count() < 15 || password.len() > 1024 {
            return Err(DataError::Validation(
                "Use at least 15 characters and at most 1024 UTF-8 bytes for the password".into(),
            )
            .into());
        }
        Ok(())
    }

    pub async fn hash(&self, password: String) -> Result<String, ApplicationError> {
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

    pub async fn verify(&self, password: String, stored: Option<String>) -> Result<bool, ApplicationError> {
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
            if stored.starts_with("$2") {
                bcrypt::verify(password, &stored).unwrap_or(false)
            } else {
                PasswordHash::new(&stored)
                    .is_ok_and(|hash| Argon2::default().verify_password(password.as_bytes(), &hash).is_ok())
            }
        })
        .await
        .map_err(|_| AuthenticationError::Hash("Password worker failed".into()))?;
        Ok(real && valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod validate {
        use super::*;
        #[test]
        fn accepts_passphrases_and_unicode_but_rejects_short_or_oversized_values() {
            assert!(PasswordService::validate("a long password phrase").is_ok());
            assert!(PasswordService::validate(&"é".repeat(15)).is_ok());
            assert!(PasswordService::validate(&"é".repeat(14)).is_err());
            assert!(PasswordService::validate(&"x".repeat(1025)).is_err());
        }
    }

    mod verify {
        use super::*;
        #[tokio::test]
        async fn supports_legacy_bcrypt_without_accepting_an_incorrect_password() {
            let service = PasswordService::new();
            let hash = bcrypt::hash("legacy-password", 4).unwrap();
            assert!(service
                .verify("legacy-password".into(), Some(hash.clone()))
                .await
                .unwrap());
            assert!(!service.verify("wrong-password".into(), Some(hash)).await.unwrap());
        }
        #[tokio::test]
        async fn hashes_the_whole_passphrase_instead_of_truncating_at_bcrypts_limit() {
            let service = PasswordService::new();
            let password = format!("{}first", "x".repeat(72));
            let hash = service.hash(password.clone()).await.unwrap();
            assert!(hash.starts_with("$argon2id$"));
            assert!(service.verify(password, Some(hash.clone())).await.unwrap());
            assert!(!service
                .verify(format!("{}second", "x".repeat(72)), Some(hash))
                .await
                .unwrap());
        }
        #[tokio::test]
        async fn rejects_missing_credentials_and_malformed_hashes() {
            let service = PasswordService::new();
            assert!(!service.verify("anything".into(), None).await.unwrap());
            assert!(!service
                .verify("anything".into(), Some("malformed".into()))
                .await
                .unwrap());
        }
    }
}

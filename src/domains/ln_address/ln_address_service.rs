use async_trait::async_trait;
use nostr_sdk::PublicKey;
use regex::Regex;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        dtos::UpdateLnAddressRequest,
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::ln_address::entities::{LnAddress, LnAddressFilter},
};

use super::LnAddressUseCases;

const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

pub struct LnAddressService {
    store: AppStore,
}

impl LnAddressService {
    pub fn new(store: AppStore) -> Self {
        LnAddressService { store }
    }
}

#[async_trait]
impl LnAddressUseCases for LnAddressService {
    async fn register(
        &self,
        wallet_id: Uuid,
        mut username: String,
        allows_nostr: bool,
        nostr_pubkey: Option<PublicKey>,
    ) -> Result<LnAddress, ApplicationError> {
        debug!(%wallet_id, username, "Registering lightning address");

        username = username.to_lowercase();
        validate_username(username.as_str())?;

        if self.store.ln_address.find_by_wallet_id(wallet_id).await?.is_some() {
            return Err(DataError::Conflict("Duplicate User ID.".to_string()).into());
        }

        if self.store.ln_address.find_by_username(&username).await?.is_some() {
            return Err(DataError::Conflict("Duplicate username.".to_string()).into());
        }

        let ln_address = self
            .store
            .ln_address
            .insert(wallet_id, &username, allows_nostr, nostr_pubkey)
            .await?;

        info!(
            %wallet_id,
            username, "Lightning address registered successfully"
        );
        Ok(ln_address)
    }

    async fn get(&self, id: Uuid) -> Result<LnAddress, ApplicationError> {
        trace!(%id, "Fetching lightning address");

        let ln_address = self
            .store
            .ln_address
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        debug!(
            %id, "Lightning address fetched successfully"
        );
        Ok(ln_address)
    }

    async fn list(&self, filter: LnAddressFilter) -> Result<Vec<LnAddress>, ApplicationError> {
        trace!(?filter, "Listing lightning addresses");

        let ln_addresses = self.store.ln_address.find_many(filter.clone()).await?;

        debug!(?filter, "Lightning addresses listed successfully");
        Ok(ln_addresses)
    }

    async fn update(&self, id: Uuid, request: UpdateLnAddressRequest) -> Result<LnAddress, ApplicationError> {
        debug!(%id, ?request, "Updating lightning address");

        let mut ln_address = self
            .store
            .ln_address
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        if let Some(mut username) = request.username {
            username = username.to_lowercase();

            if username != ln_address.username {
                validate_username(username.as_str())?;

                if self.store.ln_address.find_by_username(&username).await?.is_some() {
                    return Err(DataError::Conflict("Duplicate username.".to_string()).into());
                }

                ln_address.username = username;
            }
        }

        if let Some(active) = request.active {
            ln_address.active = active;
        }

        if let Some(allows_nostr) = request.allows_nostr {
            ln_address.allows_nostr = allows_nostr;
        }

        if let Some(nostr_pubkey) = request.nostr_pubkey {
            ln_address.nostr_pubkey = Some(nostr_pubkey);
        }

        let ln_address = self.store.ln_address.update(ln_address).await?;

        info!(%id, "Lightning address updated successfully");
        Ok(ln_address)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting lightning address");

        let n_deleted = self
            .store
            .ln_address
            .delete_many(LnAddressFilter {
                ids: Some(vec![id]),
                ..Default::default()
            })
            .await?;

        if n_deleted == 0 {
            return Err(DataError::NotFound("Lightning address not found.".to_string()).into());
        }

        info!(%id, "Lightning address deleted successfully");
        Ok(())
    }

    async fn delete_many(&self, filter: LnAddressFilter) -> Result<u64, ApplicationError> {
        debug!(?filter, "Deleting lightning addresses");

        let n_deleted = self.store.ln_address.delete_many(filter.clone()).await?;

        info!(?filter, n_deleted, "Lightning addresses deleted successfully");
        Ok(n_deleted)
    }
}

fn validate_username(username: &str) -> Result<(), DataError> {
    if username.len() < MIN_USERNAME_LENGTH || username.len() > MAX_USERNAME_LENGTH {
        return Err(DataError::Validation("Invalid username length.".to_string()));
    }

    // Regex validation for allowed characters in username
    let email_username_re = Regex::new(r"^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+$").expect("should not fail as a constant");
    if !email_username_re.is_match(username) {
        return Err(DataError::Validation("Invalid username format.".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::entities::AppStore,
        domains::ln_address::{LnAddress, LnAddressFilter, MockLnAddressRepository},
    };
    use chrono::Utc;
    use sea_orm::DatabaseConnection;
    use std::sync::Arc;
    use uuid::Uuid;

    fn mock_store(repo: MockLnAddressRepository) -> AppStore {
        let mut store = AppStore::new_sea_orm(DatabaseConnection::default());
        store.ln_address = Arc::new(repo);
        store
    }

    #[tokio::test]
    async fn register_creates_address() {
        let wallet_id = Uuid::new_v4();
        let username = "alice".to_string();

        let expected = LnAddress {
            id: Uuid::new_v4(),
            wallet_id,
            username: username.clone(),
            active: true,
            allows_nostr: false,
            nostr_pubkey: None,
            created_at: Utc::now(),
            updated_at: None,
        };

        let mut repo = MockLnAddressRepository::new();
        repo.expect_find_by_wallet_id()
            .withf(move |id| *id == wallet_id)
            .return_once(|_| Ok(None));
        let user_check = username.clone();
        repo.expect_find_by_username()
            .withf(move |u| u == user_check)
            .return_once(|_| Ok(None));
        let inserted = expected.clone();
        repo.expect_insert()
            .withf(move |id, user, allows_nostr, pk| {
                *id == wallet_id && user == "alice" && !allows_nostr && pk.is_none()
            })
            .return_once(move |_, _, _, _| Ok(inserted));

        let store = mock_store(repo);
        let service = LnAddressService::new(store);
        let result = service
            .register(wallet_id, username.clone(), false, None)
            .await
            .unwrap();

        assert_eq!(result.wallet_id, wallet_id);
        assert_eq!(result.username, username);
    }

    #[tokio::test]
    async fn get_returns_address() {
        let id = Uuid::new_v4();
        let address = LnAddress {
            id,
            wallet_id: Uuid::new_v4(),
            username: "bob".into(),
            active: true,
            allows_nostr: false,
            nostr_pubkey: None,
            created_at: Utc::now(),
            updated_at: None,
        };

        let mut repo = MockLnAddressRepository::new();
        repo.expect_find()
            .withf(move |uid| *uid == id)
            .return_once(move |_| Ok(Some(address.clone())));

        let store = mock_store(repo);
        let service = LnAddressService::new(store);
        let result = service.get(id).await.unwrap();

        assert_eq!(result.id, id);
    }

    #[tokio::test]
    async fn list_calls_repository() {
        let filter = LnAddressFilter {
            username: Some("charlie".into()),
            ..Default::default()
        };
        let address = LnAddress {
            id: Uuid::new_v4(),
            wallet_id: Uuid::new_v4(),
            username: "charlie".into(),
            active: true,
            allows_nostr: false,
            nostr_pubkey: None,
            created_at: Utc::now(),
            updated_at: None,
        };

        let mut repo = MockLnAddressRepository::new();
        let expected_filter = filter.clone();
        repo.expect_find_many()
            .withf(move |f| f.username == expected_filter.username)
            .return_once(move |_| Ok(vec![address.clone()]));

        let store = mock_store(repo);
        let service = LnAddressService::new(store);
        let list = service.list(filter.clone()).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].username, "charlie");
    }

    #[tokio::test]
    async fn update_updates_address() {
        let id = Uuid::new_v4();
        let mut existing = LnAddress {
            id,
            wallet_id: Uuid::new_v4(),
            username: "dan".into(),
            active: true,
            allows_nostr: false,
            nostr_pubkey: None,
            created_at: Utc::now(),
            updated_at: None,
        };

        let mut repo = MockLnAddressRepository::new();
        repo.expect_find()
            .withf(move |uid| *uid == id)
            .return_once(move |_| Ok(Some(existing.clone())));
        repo.expect_find_by_username()
            .withf(|u| u == "eve")
            .return_once(|_| Ok(None));
        repo.expect_update()
            .withf(|ln| ln.username == "eve" && !ln.active)
            .return_once(|ln| Ok(ln));

        let store = mock_store(repo);
        let service = LnAddressService::new(store);
        let result = service
            .update(
                id,
                UpdateLnAddressRequest {
                    username: Some("eve".into()),
                    active: Some(false),
                    allows_nostr: None,
                    nostr_pubkey: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.username, "eve");
        assert!(!result.active);
    }

    #[tokio::test]
    async fn delete_deletes_address() {
        let id = Uuid::new_v4();
        let mut repo = MockLnAddressRepository::new();
        repo.expect_delete_many()
            .withf(move |f| f.ids.as_ref().unwrap() == &[id])
            .return_once(|_| Ok(1));

        let store = mock_store(repo);
        let service = LnAddressService::new(store);
        service.delete(id).await.unwrap();
    }

    #[tokio::test]
    async fn delete_many_calls_repository() {
        let filter = LnAddressFilter {
            active: Some(false),
            ..Default::default()
        };

        let mut repo = MockLnAddressRepository::new();
        let expected = filter.clone();
        repo.expect_delete_many()
            .withf(move |f| f.active == expected.active)
            .return_once(|_| Ok(2));

        let store = mock_store(repo);
        let service = LnAddressService::new(store);
        let n = service.delete_many(filter.clone()).await.unwrap();
        assert_eq!(n, 2);
    }
}

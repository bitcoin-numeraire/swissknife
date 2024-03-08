use async_trait::async_trait;
use regex::Regex;
use tracing::{info, trace};

use crate::{
    application::errors::{ApplicationError, DataError, LightningError},
    domains::{
        lightning::{
            entities::{LNURLp, LightningAddress},
            usecases::LightningAddressesUseCases,
        },
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

const MAX_SENDABLE: u64 = 1000000000;
const MIN_SENDABLE: u64 = 1000;
const MAX_COMMENT_CHARS: u8 = 255;
const LNURL_TYPE: &str = "payRequest";
const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

#[async_trait]
impl LightningAddressesUseCases for LightningService {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLp, ApplicationError> {
        trace!(username, "Generating LNURLp");

        let lightning_address = self.store.get_by_username(&username).await?;
        if lightning_address.is_none() {
            return Err(DataError::NotFound("Lightning address not found.".into()).into());
        }
        let metadata = generate_lnurlp_metadata(&username, &self.domain)?;

        let lnurlp = LNURLp {
            callback: format!(
                "https://{}/lightning/lnurlp/{}/callback",
                self.domain, username
            ),
            max_sendable: MAX_SENDABLE,
            min_sendable: MIN_SENDABLE,
            metadata,
            comment_allowed: Some(MAX_COMMENT_CHARS),
            withdraw_link: None,
            tag: LNURL_TYPE.to_string(),
        };

        info!(username, "LNURLp returned successfully");
        Ok(lnurlp)
    }

    async fn generate_invoice(
        &self,
        username: String,
        amount: u64,
    ) -> Result<String, ApplicationError> {
        trace!(username, "Generating lightning invoice");

        let lightning_address = self.store.get_by_username(&username).await?;
        if lightning_address.is_none() {
            return Err(DataError::NotFound("Lightning address not found.".into()).into());
        }

        let metadata = generate_lnurlp_metadata(&username, &self.domain)?;
        let invoice = self.lightning_client.invoice(amount, metadata).await?;

        info!(username, "Lightning invoice generated successfully");
        Ok(invoice)
    }

    async fn register_lightning_address(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError> {
        trace!(
            user_id = user.sub,
            username,
            "Registering lightning address"
        );

        // Length check
        let username_length = username.len();
        if username_length < MIN_USERNAME_LENGTH || username_length > MAX_USERNAME_LENGTH {
            return Err(DataError::Validation("Invlaid username length.".to_string()).into());
        }

        // Regex validation for allowed characters
        let email_username_re = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$").unwrap(); // Can't fail by assertion
        if !email_username_re.is_match(&username) {
            return Err(DataError::Validation("Invalid username format.".to_string()).into());
        }

        if let Some(_) = self.store.get_by_user_id(&user.sub).await? {
            return Err(DataError::Conflict(
                "User has already registered a lightning address.".to_string(),
            )
            .into());
        }

        if let Some(_) = self.store.get_by_username(&username).await? {
            return Err(DataError::Conflict("Username already exists.".to_string()).into());
        }

        let lightning_address = self.store.insert(&user.sub, &username).await?;

        info!(
            user_id = user.sub,
            username, "Lightning address registered successfully"
        );
        Ok(lightning_address)
    }

    async fn get_lightning_address(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError> {
        trace!(user_id = user.sub, "Fetching lightning address");

        let lightning_address = self.store.get_by_username(&username).await?;

        match lightning_address {
            Some(addr) if addr.user_id == user.sub => {
                // The user is accessing their own address, no extra permission needed
                info!(user_id = user.sub, "Lightning address fetched successfully");
                Ok(addr)
            }
            Some(addr) => {
                // Here, the user is trying to access someone else's address
                // Check if the user has the permission to view all lightning address
                user.check_permission(Permission::ReadLightningAddress)?;
                info!(
                    user_id = user.sub,
                    "Lightning address fetched successfully for another user"
                );
                Ok(addr)
            }
            None => Err(DataError::NotFound("Lightning address not found.".into()).into()),
        }
    }

    async fn list_lightning_addresses(
        &self,
        user: AuthUser,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<LightningAddress>, ApplicationError> {
        trace!(
            user_id = user.sub,
            limit,
            offset,
            "Listing lightning addresses"
        );

        let lightning_addresses = if user.permissions.contains(&Permission::ReadLightningAddress) {
            // The user has permission to view all addresses
            self.store.list("", limit, offset).await?
        } else {
            // The user can only view their own addresses
            self.store.list(&user.sub, limit, offset).await?
        };

        info!(
            user_id = user.sub,
            "Lightning addresses listed successfully"
        );
        Ok(lightning_addresses)
    }
}

fn generate_lnurlp_metadata(username: &str, domain: &str) -> Result<String, LightningError> {
    serde_json::to_string(&[
        [
            "text/plain".to_string(),
            format!("{} never refuses sats", username),
        ],
        [
            "text/identifier".to_string(),
            format!("{}@{}", username, domain),
        ],
    ])
    .map_err(|e| LightningError::ParseMetadata(e.to_string()))
}

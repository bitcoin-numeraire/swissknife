use async_trait::async_trait;
use regex::Regex;
use tracing::{debug, info};

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{
        lightning::{
            entities::{
                LNURLPayRequest, LightningAddress, LightningInvoice, LightningInvoiceStatus,
            },
            usecases::LightningAddressesUseCases,
        },
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

#[async_trait]
impl LightningAddressesUseCases for LightningService {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLPayRequest, ApplicationError> {
        debug!(username, "Generating LNURLp");

        self.store
            .find_address_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        info!(username, "LNURLp returned successfully");
        Ok(LNURLPayRequest::new(&username, &self.domain))
    }

    async fn generate_invoice(
        &self,
        username: String,
        amount: u64,
        description: String,
    ) -> Result<LightningInvoice, ApplicationError> {
        debug!(username, "Generating lightning invoice");

        self.store
            .find_address_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        let mut invoice = self.lightning_client.invoice(amount, description).await?;
        invoice.lightning_address = Some(username.clone());
        invoice.status = LightningInvoiceStatus::PENDING;

        let invoice = self.store.insert_invoice(invoice).await?;

        info!(username, "Lightning invoice generated successfully");
        Ok(invoice)
    }

    async fn register_lightning_address(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError> {
        debug!(
            user_id = user.sub,
            username, "Registering lightning address"
        );

        // Regex validation for allowed characters
        let email_username_re = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$").unwrap(); // Can't fail by assertion
        if !email_username_re.is_match(&username) {
            return Err(DataError::Validation("Invalid username format.".to_string()).into());
        }

        if self
            .store
            .find_address_by_user_id(&user.sub)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict(
                "User has already registered a lightning address.".to_string(),
            )
            .into());
        }

        if self
            .store
            .find_address_by_username(&username)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict("Username already exists.".to_string()).into());
        }

        let lightning_address = self.store.insert_address(&user.sub, &username).await?;

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
        debug!(user_id = user.sub, "Fetching lightning address");

        let lightning_address = self
            .store
            .find_address_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        if lightning_address.user_id != user.sub {
            user.check_permission(Permission::ReadLightningAccounts)?;
        }

        info!(user_id = user.sub, "Lightning address fetched successfully");
        Ok(lightning_address)
    }

    async fn list_lightning_addresses(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, ApplicationError> {
        debug!(
            user_id = user.sub,
            limit, offset, "Listing lightning addresses"
        );

        let lightning_addresses = if user.has_permission(Permission::ReadLightningAccounts) {
            // The user has permission to view all addresses
            self.store.find_all_addresses(None, limit, offset).await?
        } else {
            // The user can only view their own addresses
            self.store
                .find_all_addresses(Some(user.sub.clone()), limit, offset)
                .await?
        };

        info!(
            user_id = user.sub,
            "Lightning addresses listed successfully"
        );
        Ok(lightning_addresses)
    }
}

use async_trait::async_trait;
use regex::Regex;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{
        lightning::{
            entities::{LNURLPayRequest, LightningAddress, LightningInvoice},
            services::LightningAddressesUseCases,
        },
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

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

    async fn generate_lnurlp_invoice(
        &self,
        username: String,
        amount: u64,
        comment: Option<String>,
    ) -> Result<LightningInvoice, ApplicationError> {
        debug!(username, amount, comment, "Generating LNURLp invoice");

        let lightning_address = self
            .store
            .find_address_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        let comment = match comment {
            Some(comm) if comm.is_empty() => self.invoice_description.clone(),
            Some(comm) => comm,
            None => self.invoice_description.clone(),
        };

        let mut invoice = self
            .lightning_client
            .invoice(amount, comment.clone(), self.invoice_expiry)
            .await?;
        invoice.user_id = lightning_address.user_id.clone();
        invoice.lightning_address = Some(username.clone());

        // TODO: Get or add more information to make this a LNURLp invoice (like fetching a success action specific to the user)
        let invoice = self.store.insert_invoice(invoice).await?;

        info!(username, "Lightning invoice generated successfully");
        Ok(invoice)
    }

    async fn register_address(
        &self,
        user: AuthUser,
        user_id: String,
        username: String,
    ) -> Result<LightningAddress, ApplicationError> {
        debug!(
            user = user.sub,
            user_id, username, "Registering lightning address"
        );

        user.check_permission(Permission::WriteLightningAddress)?;

        if username.len() < MIN_USERNAME_LENGTH || username.len() > MAX_USERNAME_LENGTH {
            return Err(DataError::Validation("Invalid username length.".to_string()).into());
        }

        // Regex validation for allowed characters
        let email_username_re = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$").unwrap(); // Can't fail by assertion
        if !email_username_re.is_match(&username) {
            return Err(DataError::Validation("Invalid username format.".to_string()).into());
        }

        if self
            .store
            .find_address_by_user_id(&user_id)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict("Duplicate User ID.".to_string()).into());
        }

        if self
            .store
            .find_address_by_username(&username)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict("Duplicate username.".to_string()).into());
        }

        let lightning_address = self.store.insert_address(&user_id, &username).await?;

        info!(
            user = user.sub,
            user_id, username, "Lightning address registered successfully"
        );
        Ok(lightning_address)
    }

    async fn get_address(
        &self,
        user: AuthUser,
        id: Uuid,
    ) -> Result<LightningAddress, ApplicationError> {
        trace!(user_id = user.sub, %id, "Fetching lightning address");

        user.check_permission(Permission::ReadLightningAddress)?;

        let lightning_address = self
            .store
            .find_address(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        debug!(
            user_id = user.sub,
            %id, "Lightning address fetched successfully"
        );
        Ok(lightning_address)
    }

    async fn list_addresses(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, ApplicationError> {
        trace!(
            user_id = user.sub,
            limit,
            offset,
            "Listing lightning addresses"
        );

        user.check_permission(Permission::ReadLightningAddress)?;

        let lightning_addresses = self.store.find_addresses(limit, offset).await?;

        debug!(
            user_id = user.sub,
            limit, offset, "Lightning addresses listed successfully"
        );
        Ok(lightning_addresses)
    }
}

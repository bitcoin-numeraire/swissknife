use async_trait::async_trait;
use regex::Regex;
use std::sync::Arc;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::{
        invoices::entities::Invoice,
        lightning::entities::{LnAddress, LnAddressFilter, LnURLPayRequest},
    },
    infra::lightning::LnClient,
};

use super::LnAddressesUseCases;

const DEFAULT_INVOICE_EXPIRY: u32 = 3600;
const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

pub struct LnAddressService {
    domain: String,
    invoice_expiry: u32,
    store: AppStore,
    ln_client: Arc<dyn LnClient>,
}

impl LnAddressService {
    pub fn new(
        store: AppStore,
        ln_client: Arc<dyn LnClient>,
        domain: String,
        invoice_expiry: Option<u32>,
    ) -> Self {
        LnAddressService {
            store,
            ln_client,
            domain,
            invoice_expiry: invoice_expiry.unwrap_or(DEFAULT_INVOICE_EXPIRY),
        }
    }
}

#[async_trait]
impl LnAddressesUseCases for LnAddressService {
    async fn generate_lnurlp(&self, username: String) -> Result<LnURLPayRequest, ApplicationError> {
        debug!(username, "Generating LNURLp");

        self.store
            .ln_address
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        info!(username, "LNURLp returned successfully");
        Ok(LnURLPayRequest::new(&username, &self.domain))
    }

    async fn generate_lnurlp_invoice(
        &self,
        username: String,
        amount: u64,
        comment: Option<String>,
    ) -> Result<Invoice, ApplicationError> {
        debug!(username, amount, comment, "Generating LNURLp invoice");

        let lightning_address = self
            .store
            .ln_address
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        let mut invoice = self
            .ln_client
            .invoice(
                amount,
                comment.unwrap_or(format!("Payment to {}@{}", username, self.domain)),
                self.invoice_expiry,
            )
            .await?;
        invoice.user_id = lightning_address.user_id.clone();
        invoice.lightning_address = Some(lightning_address.id);

        // TODO: Get or add more information to make this a LNURLp invoice (like fetching a success action specific to the user)
        let invoice = self.store.invoice.insert(None, invoice).await?;

        info!(username, "Lightning invoice generated successfully");
        Ok(invoice)
    }

    async fn register(
        &self,
        user_id: String,
        username: String,
    ) -> Result<LnAddress, ApplicationError> {
        debug!(user_id, username, "Registering lightning address");

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
            .ln_address
            .find_by_user_id(&user_id)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict("Duplicate User ID.".to_string()).into());
        }

        if self
            .store
            .ln_address
            .find_by_username(&username)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict("Duplicate username.".to_string()).into());
        }

        let lightning_address = self.store.ln_address.insert(&user_id, &username).await?;

        info!(
            user_id,
            username, "Lightning address registered successfully"
        );
        Ok(lightning_address)
    }

    async fn get(&self, id: Uuid) -> Result<LnAddress, ApplicationError> {
        trace!(%id, "Fetching lightning address");

        let lightning_address = self
            .store
            .ln_address
            .find(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        debug!(
            %id, "Lightning address fetched successfully"
        );
        Ok(lightning_address)
    }

    async fn list(&self, filter: LnAddressFilter) -> Result<Vec<LnAddress>, ApplicationError> {
        trace!(?filter, "Listing lightning addresses");

        let lightning_addresses = self.store.ln_address.find_many(filter.clone()).await?;

        debug!(?filter, "Lightning addresses listed successfully");
        Ok(lightning_addresses)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%id, "Deleting lightning address");

        let n_deleted = self
            .store
            .ln_address
            .delete_many(LnAddressFilter {
                id: Some(id),
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

        info!(
            ?filter,
            n_deleted, "Lightning addresses deleted successfully"
        );
        Ok(n_deleted)
    }
}

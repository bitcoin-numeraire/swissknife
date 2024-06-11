use std::{sync::Arc, vec};

use async_trait::async_trait;
use breez_sdk_core::{MessageSuccessActionData, SuccessActionProcessed};
use regex::Regex;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    domains::lightning::entities::{
        LnAddress, LnAddressFilter, LnURLPayRequest, LnUrlCallbackResponse,
    },
    infra::lightning::LnClient,
};

use super::LnUrlUseCases;

const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

pub struct LnUrlService {
    domain: String,
    store: AppStore,
    invoice_expiry: u32,
    ln_client: Arc<dyn LnClient>,
}

impl LnUrlService {
    pub fn new(
        store: AppStore,
        ln_client: Arc<dyn LnClient>,
        invoice_expiry: u32,
        domain: String,
    ) -> Self {
        LnUrlService {
            store,
            ln_client,
            invoice_expiry,
            domain,
        }
    }
}

#[async_trait]
impl LnUrlUseCases for LnUrlService {
    async fn lnurlp(&self, username: String) -> Result<LnURLPayRequest, ApplicationError> {
        debug!(username, "Generating LNURLp");

        self.store
            .ln_address
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        let metadata = serde_json::to_string(&[
            [
                "text/plain".to_string(),
                format!("{} never refuses sats", username),
            ],
            [
                "text/identifier".to_string(),
                format!("{}@{}", username, self.domain),
            ],
        ])
        .unwrap();

        let lnurlp = LnURLPayRequest {
            callback: format!("https://{}/api/lnurlp/{}/callback", self.domain, username),
            max_sendable: 1000000000,
            min_sendable: 1000,
            metadata,
            comment_allowed: 255,
            tag: "payRequest".to_string(),
        };

        info!(username, "LNURLp returned successfully");
        Ok(lnurlp)
    }

    async fn lnurlp_callback(
        &self,
        username: String,
        amount: u64,
        comment: Option<String>,
    ) -> Result<LnUrlCallbackResponse, ApplicationError> {
        debug!(username, amount, comment, "Generating LNURLp invoice");

        let ln_address = self
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
        invoice.user_id = ln_address.user_id.clone();
        invoice.ln_address = Some(ln_address.id);

        // TODO: Get or add more information to make this a LNURLp invoice (like fetching a success action specific to the user)
        let invoice = self.store.invoice.insert(None, invoice).await?;
        let lnurlp_invoice = LnUrlCallbackResponse {
            pr: invoice.lightning.unwrap().bolt11,
            success_action: Some(SuccessActionProcessed::Message {
                data: MessageSuccessActionData {
                    message: "Thanks for the sats!".to_string(),
                },
            }),
            disposable: None,
            routes: vec![],
        };

        info!(username, "Lightning invoice generated successfully");
        Ok(lnurlp_invoice)
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

        let ln_address = self.store.ln_address.insert(&user_id, &username).await?;

        info!(
            user_id,
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

        info!(
            ?filter,
            n_deleted, "Lightning addresses deleted successfully"
        );
        Ok(n_deleted)
    }
}

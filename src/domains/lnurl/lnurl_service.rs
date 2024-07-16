use std::{sync::Arc, vec};

use async_trait::async_trait;
use breez_sdk_core::{MessageSuccessActionData, SuccessActionProcessed};
use tracing::{debug, info};

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    infra::lightning::LnClient,
};

use super::{LnURLPayRequest, LnUrlCallback, LnUrlUseCases};

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
        .expect("should not fail as a constant");

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
    ) -> Result<LnUrlCallback, ApplicationError> {
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
        invoice.wallet_id.clone_from(&ln_address.user_id);
        invoice.ln_address_id = Some(ln_address.id);

        // TODO: Get or add more information to make this a LNURLp invoice (like fetching a success action specific to the user)
        let invoice = self.store.invoice.insert(None, invoice).await?;
        let lnurlp_invoice = LnUrlCallback {
            pr: invoice
                .ln_invoice
                .expect("should exist for ledger Lightning")
                .bolt11,
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
}

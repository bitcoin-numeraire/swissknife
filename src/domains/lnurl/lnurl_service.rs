use std::{sync::Arc, vec};

use async_trait::async_trait;
use tracing::{debug, info};

use crate::{
    application::{
        entities::AppStore,
        errors::{ApplicationError, DataError},
    },
    infra::lightning::LnClient,
};

use super::{LnURLPayRequest, LnUrlCallback, LnUrlSuccessAction, LnUrlUseCases};

const MIN_SENDABLE: u64 = 1000;
const MAX_SENDABLE: u64 = 250000000;
const COMMENT_ALLOWED: u16 = 255;

pub struct LnUrlService {
    domain: String,
    host: String,
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
        host: String,
    ) -> Self {
        LnUrlService {
            store,
            ln_client,
            invoice_expiry,
            domain,
            host,
        }
    }

    fn metadata(&self, username: &str) -> String {
        serde_json::to_string(&[
            [
                "text/identifier".to_string(),
                format!("{}@{}", username, self.domain),
            ],
            [
                "text/plain".to_string(),
                format!("{} never refuses sats", username),
            ],
        ])
        .expect("should not fail as a constant")
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

        let lnurlp = LnURLPayRequest {
            callback: format!("{}/lnurlp/{}/callback", self.host, username),
            max_sendable: MAX_SENDABLE,
            min_sendable: MIN_SENDABLE,
            metadata: self.metadata(&username),
            comment_allowed: COMMENT_ALLOWED,
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
            .invoice(amount, self.metadata(&username), self.invoice_expiry, true)
            .await?;
        invoice.wallet_id.clone_from(&ln_address.wallet_id);
        invoice.ln_address_id = Some(ln_address.id);
        invoice.description = comment;

        // TODO: Get or add more information to make this a LNURLp invoice (like fetching a success action specific to the user)
        let invoice = self.store.invoice.insert(None, invoice).await?;
        let lnurlp_invoice = LnUrlCallback {
            pr: invoice
                .ln_invoice
                .expect("should exist for ledger Lightning")
                .bolt11,
            success_action: Some(LnUrlSuccessAction {
                tag: "message".to_string(),
                message: Some("Thanks for the sats!".to_string()),
                ..Default::default()
            }),
            disposable: None,
            routes: vec![],
        };

        info!(username, "Lightning invoice generated successfully");
        Ok(lnurlp_invoice)
    }
}

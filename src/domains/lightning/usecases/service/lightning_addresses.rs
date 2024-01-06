use async_trait::async_trait;
use tracing::{debug, info, trace};

use crate::{
    application::errors::LightningError,
    domains::lightning::{
        entities::{LNURLp, LightningAddress},
        usecases::LightningAddressesUseCases,
    },
};

use super::LightningService;

const MAX_SENDABLE: u64 = 1000000000;
const MIN_SENDABLE: u64 = 1000;
const MAX_COMMENT_CHARS: u8 = 255;
const LNURL_TYPE: &str = "payRequest";

#[async_trait]
impl LightningAddressesUseCases for LightningService {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLp, LightningError> {
        trace!(username, "Generating LNURLp");

        // TODO: Verify the username exists

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

        trace!(username, "LNURLp generated successfully");
        Ok(lnurlp)
    }

    async fn generate_invoice(
        &self,
        username: String,
        amount: u64,
    ) -> Result<String, LightningError> {
        trace!(username, "Generating lightning invoice");

        // TODO: Verify the username exists

        let metadata = generate_lnurlp_metadata(&username, &self.domain)?;
        let invoice = self.lightning_client.invoice(amount, metadata).await?;

        info!(username, "Lightning invoice generated successfully");
        Ok(invoice)
    }

    async fn register_lightning_address(
        &self,
        user_id: String,
        username: String,
    ) -> Result<LightningAddress, LightningError> {
        trace!(user_id, username, "Registering lightning address");

        // TODO: Verify the username is not already registered

        let lightning_address = sqlx::query_as!(
            LightningAddress,
            // language=PostgreSQL
            r#"
                insert into "lightning_addresses"(user_id, username)
                values ($1, $2)
                returning *
            "#,
            user_id,
            username
        )
        .fetch_one(&self.db_client.pool())
        .await
        .map_err(|e| {
            let err_message = "Database error";
            debug!(error = ?e, err_message);
            LightningError::Register(e.to_string())
        })?;

        info!(
            user_id,
            username, "Lightning address registered successfully"
        );
        Ok(lightning_address)
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
    .map_err(|e| {
        let err_message = "Failed to generate metadata for lightning invoice";
        debug!(error = ?e, err_message);
        LightningError::ParseMetadata(e.to_string())
    })
}

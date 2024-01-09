use async_trait::async_trait;
use tracing::{info, trace};

use crate::{
    application::errors::{ApplicationError, LightningError},
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

#[async_trait]
impl LightningAddressesUseCases for LightningService {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLp, ApplicationError> {
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

        info!(username, "LNURLp returned successfully");
        Ok(lnurlp)
    }

    async fn generate_invoice(
        &self,
        username: String,
        amount: u64,
    ) -> Result<String, ApplicationError> {
        trace!(username, "Generating lightning invoice");

        // TODO: Verify the username exists

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

        user.check_permission(Permission::RegisterLightningAddress)?;

        // TODO: Verify the username is not already registered

        // TODO: Implement this as a repository function
        let lightning_address = sqlx::query_as!(
            LightningAddress,
            // language=PostgreSQL
            r#"
                insert into "lightning_addresses"(user_id, username)
                values ($1, $2)
                returning *
            "#,
            user.sub,
            username
        )
        .fetch_one(&self.db_client.pool())
        .await
        .map_err(|e| LightningError::Register(e.to_string()))?;

        info!(
            user_id = user.sub,
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
    .map_err(|e| LightningError::ParseMetadata(e.to_string()))
}

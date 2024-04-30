use async_trait::async_trait;
use tracing::{debug, info};

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{
        lightning::{entities::LightningInvoice, usecases::LightningInvoicesUseCases},
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

#[async_trait]
impl LightningInvoicesUseCases for LightningService {
    async fn get_lightning_invoice(
        &self,
        user: AuthUser,
        payment_hash: String,
    ) -> Result<LightningInvoice, ApplicationError> {
        debug!(
            user_id = user.sub,
            payment_hash, "Fetching lightning invoice"
        );

        let lightning_invoice = self
            .store
            .find_invoice(&payment_hash)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning invoice not found.".to_string()))?;

        if lightning_invoice.user_id != user.sub {
            user.check_permission(Permission::ReadLightningAccounts)?;
        }

        info!(
            user_id = user.sub,
            payment_hash, "Lightning invoice fetched successfully"
        );
        Ok(lightning_invoice)
    }

    async fn list_lightning_invoices(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningInvoice>, ApplicationError> {
        debug!(
            user_id = user.sub,
            limit, offset, "Listing lightning invoices"
        );

        let lightning_invoices = if user.has_permission(Permission::ReadLightningAccounts) {
            // The user has permission to view all addresses
            self.store.find_all_invoices(None, limit, offset).await?
        } else {
            // The user can only view their own payments
            self.store
                .find_all_invoices(Some(user.sub.clone()), limit, offset)
                .await?
        };

        info!(user_id = user.sub, "Lightning invoices listed successfully");
        Ok(lightning_invoices)
    }
}
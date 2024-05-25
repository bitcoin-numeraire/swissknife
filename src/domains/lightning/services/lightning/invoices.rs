use async_trait::async_trait;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{
        lightning::{
            entities::{LightningInvoice, LightningInvoiceFilter, LightningInvoiceStatus},
            services::LightningInvoicesUseCases,
        },
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

#[async_trait]
impl LightningInvoicesUseCases for LightningService {
    async fn get_invoice(
        &self,
        user: AuthUser,
        id: Uuid,
    ) -> Result<LightningInvoice, ApplicationError> {
        trace!(
            user_id = user.sub,
            %id,
            "Fetching lightning invoice"
        );

        user.check_permission(Permission::ReadLightningTransaction)?;

        let lightning_invoice = self
            .store
            .find_invoice(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning invoice not found.".to_string()))?;

        debug!(
            user_id = user.sub,
            %id, "Lightning invoice fetched successfully"
        );
        Ok(lightning_invoice)
    }

    async fn list_invoices(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
        filter: LightningInvoiceFilter,
    ) -> Result<Vec<LightningInvoice>, ApplicationError> {
        trace!(
            user_id = user.sub,
            limit,
            offset,
            "Listing lightning invoices"
        );

        user.check_permission(Permission::ReadLightningTransaction)?;

        let lightning_invoices = self.store.find_invoices(filter).await?;

        debug!(user_id = user.sub, "Lightning invoices listed successfully");
        Ok(lightning_invoices)
    }

    async fn delete_expired_invoices(&self, user: AuthUser) -> Result<u64, ApplicationError> {
        debug!(
            user_id = user.sub,
            "Deleting all expired lightning invoices"
        );

        user.check_permission(Permission::WriteLightningTransaction)?;

        let n_deleted = self
            .store
            .delete_invoices(LightningInvoiceFilter {
                user_id: None,
                status: Some(LightningInvoiceStatus::EXPIRED),
                ..Default::default()
            })
            .await?;

        info!(
            user_id = user.sub,
            n_deleted, "All expired lightning invoices deleted successfully"
        );

        Ok(n_deleted)
    }
}

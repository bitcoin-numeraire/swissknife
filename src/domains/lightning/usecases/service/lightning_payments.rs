use async_trait::async_trait;
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{
        lightning::{entities::LightningPayment, usecases::LightningPaymentsUseCases},
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

#[async_trait]
impl LightningPaymentsUseCases for LightningService {
    async fn get_payment(
        &self,
        user: AuthUser,
        id: Uuid,
    ) -> Result<LightningPayment, ApplicationError> {
        debug!(user_id = user.sub, "Fetching lightning payment");

        let lightning_payment = self
            .store
            .find_payment(id)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning payment not found.".to_string()))?;

        if lightning_payment.user_id != user.sub {
            user.check_permission(Permission::ReadLightningAccounts)?;
        }

        info!(
            user_id = user.sub,
            id = id.to_string(),
            "Lightning payment fetched successfully"
        );
        Ok(lightning_payment)
    }

    async fn list_payments(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningPayment>, ApplicationError> {
        debug!(
            user_id = user.sub,
            limit, offset, "Listing lightning payments"
        );

        let lightning_payments = if user.has_permission(Permission::ReadLightningAccounts) {
            // The user has permission to view all addresses
            self.store.find_all_payments(None, limit, offset).await?
        } else {
            // The user can only view their own payments
            self.store
                .find_all_payments(Some(user.sub.clone()), limit, offset)
                .await?
        };

        info!(user_id = user.sub, "Lightning payments listed successfully");
        Ok(lightning_payments)
    }
}

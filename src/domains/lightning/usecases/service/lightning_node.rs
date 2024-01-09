use async_trait::async_trait;
use breez_sdk_core::{NodeState, Payment};
use tracing::{debug, trace};

use crate::{
    application::errors::ApplicationError,
    domains::{
        lightning::usecases::LightningNodeUseCases,
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

#[async_trait]
impl LightningNodeUseCases for LightningService {
    async fn node_info(&self, user: AuthUser) -> Result<NodeState, ApplicationError> {
        trace!(user_id = user.sub, "Getting node info");

        user.check_permission(Permission::ReadLightningNode)?;

        // TODO: Implement entity for node info and not NodeState
        let node_info = self.lightning_client.node_info()?;

        debug!("Node info retrieved successfully");
        Ok(node_info)
    }

    async fn list_payments(&self, user: AuthUser) -> Result<Vec<Payment>, ApplicationError> {
        trace!(user_id = user.sub, "Listing payments");

        user.check_permission(Permission::ReadLightningNode)?;

        // TODO: Implement entity for payments
        let payments = self.lightning_client.list_payments().await?;

        debug!("Payments retrieved successfully from node");
        Ok(payments)
    }
}

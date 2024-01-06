use async_trait::async_trait;
use breez_sdk_core::{NodeState, Payment};
use tracing::{debug, trace};

use crate::{
    application::errors::LightningError, domains::lightning::usecases::LightningNodeUseCases,
};

use super::LightningService;

#[async_trait]
impl LightningNodeUseCases for LightningService {
    async fn node_info(&self, user_id: String) -> Result<NodeState, LightningError> {
        trace!(user_id, "Getting node info");

        // TODO: RBAC

        // TODO: Implement entity for node info and not NodeState
        let node_info = self.lightning_client.node_info().await?;

        debug!("Node info retrieved successfully");
        Ok(node_info)
    }

    async fn list_payments(&self, user_id: String) -> Result<Vec<Payment>, LightningError> {
        trace!(user_id, "Listing payments");

        // TODO: RBAC

        // TODO: Implement entity for payments
        let payments = self.lightning_client.list_payments().await?;

        debug!("Payments retrieved successfully from node");
        Ok(payments)
    }
}

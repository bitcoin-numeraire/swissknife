use async_trait::async_trait;
use breez_sdk_core::{LspInformation, NodeState, Payment};
use tracing::{debug, info, trace};

use crate::{
    application::errors::ApplicationError,
    domains::{
        lightning::{entities::LightningPayment, usecases::LightningNodeUseCases},
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

    async fn lsp_info(&self, user: AuthUser) -> Result<LspInformation, ApplicationError> {
        trace!(user_id = user.sub, "Getting LSP info");

        user.check_permission(Permission::ReadLightningNode)?;

        // TODO: Implement entity for LSP info and not LspInformation
        let lsp_info = self.lightning_client.lsp_info().await?;

        debug!("LSP info retrieved successfully");
        Ok(lsp_info)
    }

    async fn list_payments(&self, user: AuthUser) -> Result<Vec<Payment>, ApplicationError> {
        trace!(user_id = user.sub, "Listing payments");

        user.check_permission(Permission::ReadLightningNode)?;

        // TODO: Implement entity for payments
        let payments = self.lightning_client.list_payments().await?;

        debug!("Payments retrieved successfully from node");
        Ok(payments)
    }

    async fn send_bolt11_payment(
        &self,
        user: AuthUser,
        bolt11: String,
        amount_msat: Option<u64>,
    ) -> Result<LightningPayment, ApplicationError> {
        trace!(
            user_id = user.sub,
            bolt11,
            "Sending payment to bolt11 invoice"
        );

        user.check_permission(Permission::SendLightningPayment)?;

        let payment = self
            .lightning_client
            .send_payment(bolt11.clone(), amount_msat)
            .await?;

        info!(user_id = user.sub, bolt11, "Payment sent successfully");
        Ok(payment)
    }
}

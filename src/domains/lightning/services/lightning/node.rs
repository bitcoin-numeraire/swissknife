use async_trait::async_trait;
use breez_sdk_core::{
    LspInformation, NodeState, Payment, ReverseSwapInfo, ServiceHealthCheckResponse,
};
use tracing::{debug, info, trace};

use crate::{
    application::errors::ApplicationError,
    domains::{
        lightning::services::LightningNodeUseCases,
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

    async fn list_lsps(&self, user: AuthUser) -> Result<Vec<LspInformation>, ApplicationError> {
        trace!(user_id = user.sub, "Listing available LSPs");

        user.check_permission(Permission::ReadLightningNode)?;

        // TODO: Implement entity for LSP info and not LspInformation
        let lsps = self.lightning_client.list_lsps().await?;

        debug!("LSPs retrieved successfully");
        Ok(lsps)
    }

    async fn list_node_payments(&self, user: AuthUser) -> Result<Vec<Payment>, ApplicationError> {
        trace!(user_id = user.sub, "Listing payments");

        user.check_permission(Permission::ReadLightningNode)?;

        // TODO: Implement entity for payments
        let payments = self.lightning_client.list_payments().await?;

        debug!("Payments retrieved successfully from node");
        Ok(payments)
    }

    async fn close_lsp_channels(&self, user: AuthUser) -> Result<Vec<String>, ApplicationError> {
        debug!(user_id = user.sub, "Closing LSP channels");

        user.check_permission(Permission::WriteLightningNode)?;

        let tx_ids = self.lightning_client.close_lsp_channels().await?;

        info!(?tx_ids, "LSP Channels closed sucessfully");
        Ok(tx_ids)
    }

    async fn pay_onchain(
        &self,
        user: AuthUser,
        amount_sat: u64,
        recipient_address: String,
        feerate: u32,
    ) -> Result<ReverseSwapInfo, ApplicationError> {
        debug!(user_id = user.sub, "Initiating on-chain payment");

        user.check_permission(Permission::WriteLightningNode)?;

        let payment_info = self
            .lightning_client
            .pay_onchain(amount_sat, recipient_address, feerate)
            .await?;

        info!("Onchain payment initiated successfully");
        Ok(payment_info)
    }

    async fn redeem(
        &self,
        user: AuthUser,
        to_address: String,
        feerate: u32,
    ) -> Result<String, ApplicationError> {
        debug!(user_id = user.sub, "Initiating on-chain redemption");

        user.check_permission(Permission::WriteLightningNode)?;

        let txid = self
            .lightning_client
            .redeem_onchain(to_address, feerate)
            .await?;

        info!("Onchain redemption initiated successfully");
        Ok(txid)
    }

    async fn health_check(
        &self,
        user: AuthUser,
    ) -> Result<ServiceHealthCheckResponse, ApplicationError> {
        trace!(user_id = user.sub, "Checking health of lightning service");

        user.check_permission(Permission::ReadLightningNode)?;

        let health = self.lightning_client.health().await?;

        Ok(health)
    }
}
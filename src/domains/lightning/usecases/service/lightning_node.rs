use async_trait::async_trait;
use breez_sdk_core::{
    parse, InputType, LspInformation, NodeState, Payment, ServiceHealthCheckResponse,
};
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError, LightningError},
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

    async fn list_node_payments(&self, user: AuthUser) -> Result<Vec<Payment>, ApplicationError> {
        trace!(user_id = user.sub, "Listing payments");

        user.check_permission(Permission::ReadLightningNode)?;

        // TODO: Implement entity for payments
        let payments = self.lightning_client.list_payments().await?;

        debug!("Payments retrieved successfully from node");
        Ok(payments)
    }

    async fn send_payment(
        &self,
        user: AuthUser,
        input: String,
        amount_msat: Option<u64>,
        comment: Option<String>,
    ) -> Result<LightningPayment, ApplicationError> {
        debug!(user_id = user.sub, input, "Sending payment from node");

        user.check_permission(Permission::SendLightningPayment)?;

        // TODO: After moving to using User_id instead of username, use send_payment function here as well by saving the payment in DB
        // associating it with the admin user and assigning its uuid to the label, to be found on event of payment success

        let input_type = parse(&input)
            .await
            .map_err(|e| DataError::Validation(e.to_string()))?;

        let id = Uuid::new_v4();

        let payment = match input_type {
            InputType::Bolt11 { invoice } => {
                self.lightning_client
                    .send_payment(invoice.bolt11, amount_msat, id)
                    .await
            }
            InputType::LnUrlPay { data } => {
                let amount = LightningService::validate_amount(amount_msat)?;
                self.lightning_client
                    .lnurl_pay(data, amount, comment, id)
                    .await
            }
            InputType::LnUrlError { data } => Err(LightningError::SendLNURLPayment(data.reason)),
            _ => Err(LightningError::UnsupportedPaymentInput(input.clone())),
        }?;

        info!(
            user_id = user.sub,
            input, "Payment sent successfully from node"
        );

        Ok(payment)
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

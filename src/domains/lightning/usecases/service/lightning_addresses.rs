use async_trait::async_trait;
use breez_sdk_core::{parse, InputType};
use regex::Regex;
use tracing::{info, trace};

use crate::{
    application::errors::{ApplicationError, DataError},
    domains::{
        lightning::{
            entities::{LNURLPayRequest, LightningAddress, LightningInvoice, LightningPayment},
            usecases::LightningAddressesUseCases,
        },
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

// TODO: Move to application layer (Bad request if not compliant)
const MIN_USERNAME_LENGTH: usize = 1;
const MAX_USERNAME_LENGTH: usize = 64;

#[async_trait]
impl LightningAddressesUseCases for LightningService {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLPayRequest, ApplicationError> {
        trace!(username, "Generating LNURLp");

        let lightning_address = self.address_repo.get_by_username(&username).await?;
        if lightning_address.is_none() {
            return Err(DataError::NotFound("Lightning address not found.".into()).into());
        }

        info!(username, "LNURLp returned successfully");
        Ok(LNURLPayRequest::new(&username, &self.domain))
    }

    async fn generate_invoice(
        &self,
        username: String,
        amount: u64,
        description: Option<String>,
    ) -> Result<LightningInvoice, ApplicationError> {
        trace!(username, "Generating lightning invoice");

        let lightning_address = self.address_repo.get_by_username(&username).await?;
        if lightning_address.is_none() {
            return Err(DataError::NotFound("Lightning address not found.".into()).into());
        }

        let mut invoice = self.lightning_client.invoice(amount, description).await?;

        invoice.lightning_address = Some(username.clone());
        invoice = self.invoice_repo.insert(invoice).await?;

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

        // Length check
        let username_length = username.len();
        if username_length < MIN_USERNAME_LENGTH || username_length > MAX_USERNAME_LENGTH {
            return Err(DataError::Validation("Invlaid username length.".to_string()).into());
        }

        // Regex validation for allowed characters
        let email_username_re = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$").unwrap(); // Can't fail by assertion
        if !email_username_re.is_match(&username) {
            return Err(DataError::Validation("Invalid username format.".to_string()).into());
        }

        if let Some(_) = self.address_repo.get_by_user_id(&user.sub).await? {
            return Err(DataError::Conflict(
                "User has already registered a lightning address.".to_string(),
            )
            .into());
        }

        if let Some(_) = self.address_repo.get_by_username(&username).await? {
            return Err(DataError::Conflict("Username already exists.".to_string()).into());
        }

        let lightning_address = self.address_repo.insert(&user.sub, &username).await?;

        info!(
            user_id = user.sub,
            username, "Lightning address registered successfully"
        );
        Ok(lightning_address)
    }

    async fn get_lightning_address(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError> {
        trace!(user_id = user.sub, "Fetching lightning address");

        let lightning_address = self.address_repo.get_by_username(&username).await?;

        match lightning_address {
            Some(addr) if addr.user_id == user.sub => {
                // The user is accessing their own address, no extra permission needed
                info!(user_id = user.sub, "Lightning address fetched successfully");
                Ok(addr)
            }
            Some(addr) => {
                // Here, the user is trying to access someone else's address
                // Check if the user has the permission to view all lightning address
                user.check_permission(Permission::ReadLightningAddress)?;
                info!(
                    user_id = user.sub,
                    "Lightning address fetched successfully by authorized user"
                );
                Ok(addr)
            }
            None => Err(DataError::NotFound("Lightning address not found.".into()).into()),
        }
    }

    async fn list_lightning_addresses(
        &self,
        user: AuthUser,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<LightningAddress>, ApplicationError> {
        trace!(
            user_id = user.sub,
            limit,
            offset,
            "Listing lightning addresses"
        );

        let lightning_addresses = if user.has_permission(Permission::ReadLightningAddress) {
            // The user has permission to view all addresses
            self.address_repo.list(limit, offset).await?
        } else {
            // The user can only view their own addresses
            self.address_repo
                .list_by_user_id(&user.sub, limit, offset)
                .await?
        };

        info!(
            user_id = user.sub,
            "Lightning addresses listed successfully"
        );
        Ok(lightning_addresses)
    }

    async fn send_payment(
        &self,
        user: AuthUser,
        input: String,
        amount_msat: Option<u64>,
    ) -> Result<LightningPayment, ApplicationError> {
        trace!(user_id = user.sub, input, "Sending payment");

        let lightning_address = self.address_repo.get_by_user_id(&user.sub).await?;
        if lightning_address.is_none() {
            return Err(DataError::NotFound("Lightning address not found.".into()).into());
        }

        // Here we can directly use Breez parsing function, no need to go through an adapter as this follows a standard
        // and is therefore part of the business layer
        let input_type = parse(&input)
            .await
            .map_err(|e| DataError::Validation(e.to_string()))?;

        match input_type {
            InputType::Bolt11 { invoice } => {
                println!(
                    "invoice amount_msat vs passed amount_msat: {} vs {}",
                    invoice.amount_msat.unwrap_or(0),
                    amount_msat.unwrap_or(0)
                );

                let payment = self
                    .lightning_client
                    .send_payment(invoice.bolt11.clone(), amount_msat)
                    .await?;

                info!(
                    user_id = user.sub,
                    bolt11 = invoice.bolt11,
                    amount_msat,
                    "Bolt11 payment sent successfully"
                );
                Ok(payment)
            }
            InputType::LnUrlPay { data } => {
                let payment = self
                    .lightning_client
                    .send_payment(data, amount_msat)
                    .await?;

                info!(
                    user_id = user.sub,
                    bolt11 = invoice.bolt11,
                    amount_msat,
                    "Bolt11 payment sent successfully"
                );
                Ok(payment)
            }
            InputType::NodeId { node_id } => {
                let amount = amount_msat.ok_or_else(|| {
                    DataError::Validation(
                        "amount_msat must be defined for spontaneous payments".to_string(),
                    )
                })?;
                if amount <= 0 {
                    return Err(DataError::Validation(
                        "amount_msat must be greater than 0 for spontaneous payments".to_string(),
                    )
                    .into());
                }

                let payment = self
                    .lightning_client
                    .send_spontaneous_payment(node_id.clone(), amount)
                    .await?;

                info!(
                    user_id = user.sub,
                    node_id, amount_msat, "Payment sent to node successfully"
                );
                Ok(payment)
            }
            InputType::BitcoinAddress { address: _ } => Err(DataError::Validation(
                "Sending to Bitcoin addresses is not yet supported. Coming soon".to_string(),
            )
            .into()),
            InputType::LnUrlError { data } => Err(DataError::Validation(format!(
                "LNURL error from beneficiary: {}",
                data.reason
            ))
            .into()),
            _ => Err(DataError::Validation("unsupported payment format".to_string()).into()),
        }
    }
}

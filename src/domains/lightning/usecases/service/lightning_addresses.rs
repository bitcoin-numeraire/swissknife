use async_trait::async_trait;
use breez_sdk_core::{parse, InputType};
use sea_orm::TransactionTrait;
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError, LightningError},
    domains::{
        lightning::{
            entities::{
                LNURLPayRequest, LightningAddress, LightningInvoice, LightningPayment, UserBalance,
            },
            usecases::LightningAddressesUseCases,
        },
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

#[async_trait]
impl LightningAddressesUseCases for LightningService {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLPayRequest, ApplicationError> {
        debug!(username, "Generating LNURLp");

        self.store
            .address_repo
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        info!(username, "LNURLp returned successfully");
        Ok(LNURLPayRequest::new(&username, &self.domain))
    }

    async fn generate_invoice(
        &self,
        username: String,
        amount: u64,
        description: String,
    ) -> Result<LightningInvoice, ApplicationError> {
        debug!(username, "Generating lightning invoice");

        self.store
            .address_repo
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        let mut invoice = self.lightning_client.invoice(amount, description).await?;
        invoice.lightning_address = Some(username.clone());
        invoice.status = "PENDING".to_string();
        invoice.id = Uuid::new_v4();

        let invoice = self.store.invoice_repo.insert(invoice).await?;

        info!(username, "Lightning invoice generated successfully");
        Ok(invoice)
    }

    async fn register_lightning_address(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<LightningAddress, ApplicationError> {
        debug!(
            user_id = user.sub,
            username, "Registering lightning address"
        );

        if self
            .store
            .address_repo
            .find_by_username(&user.sub)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict(
                "User has already registered a lightning address.".to_string(),
            )
            .into());
        }

        if self
            .store
            .address_repo
            .find_by_username(&username)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict("Username already exists.".to_string()).into());
        }

        let lightning_address = self.store.address_repo.insert(&user.sub, &username).await?;

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
        debug!(user_id = user.sub, "Fetching lightning address");

        let lightning_address = self
            .store
            .address_repo
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        if lightning_address.user_id != user.sub {
            user.check_permission(Permission::ReadLightningAddress)?;
        }

        info!(user_id = user.sub, "Lightning address fetched successfully");
        Ok(lightning_address)
    }

    async fn list_lightning_addresses(
        &self,
        user: AuthUser,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<LightningAddress>, ApplicationError> {
        debug!(
            user_id = user.sub,
            limit, offset, "Listing lightning addresses"
        );

        let lightning_addresses = if user.has_permission(Permission::ReadLightningAddress) {
            // The user has permission to view all addresses
            self.store.address_repo.find_all(limit, offset).await?
        } else {
            // The user can only view their own addresses
            self.store
                .address_repo
                .find_all_by_user_id(&user.sub, limit, offset)
                .await?
        };

        info!(
            user_id = user.sub,
            "Lightning addresses listed successfully"
        );
        Ok(lightning_addresses)
    }

    async fn get_balance(
        &self,
        user: AuthUser,
        username: String,
    ) -> Result<UserBalance, ApplicationError> {
        debug!(user_id = user.sub, "Fetching balance");

        let lightning_address = self
            .store
            .address_repo
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        if lightning_address.user_id != user.sub {
            user.check_permission(Permission::ReadLightningAddress)?;
        }

        let balance = self
            .store
            .address_repo
            .get_balance_by_username(&username)
            .await?;

        info!(user_id = user.sub, "Balance fetched successfully");
        Ok(balance)
    }

    async fn send_payment(
        &self,
        user: AuthUser,
        input: String,
        amount_msat: Option<u64>,
        comment: Option<String>,
    ) -> Result<LightningPayment, ApplicationError> {
        debug!(user_id = user.sub, input, "Sending payment");

        let ln_address = self
            .store
            .address_repo
            .find_by_user_id(&user.sub)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        let txn = self
            .store
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        // TODO: Create a PENDING payment before actually sending it so we reduce the available amount for concurrent requests
        // TODO: Add UUID to labels and only encapsulate the insert PENDING in a transaction
        let balance = self
            .store
            .address_repo
            .get_balance_by_username(&ln_address.username)
            .await?;

        if balance.available_msat < amount_msat.unwrap_or_default() as i64 {
            return Err(LightningError::InsufficientFunds.into());
        }

        let input_type = parse(&input)
            .await
            .map_err(|e| DataError::Validation(e.to_string()))?;

        let mut payment = match input_type {
            InputType::Bolt11 { invoice } => {
                self.lightning_client
                    .send_payment(invoice.bolt11.clone(), amount_msat)
                    .await
            }
            InputType::LnUrlPay { data } => {
                let amount = validate_amount(amount_msat)?;
                self.lightning_client.lnurl_pay(data, amount, comment).await
            }
            InputType::NodeId { node_id } => {
                let amount = validate_amount(amount_msat)?;
                self.lightning_client
                    .send_spontaneous_payment(node_id.clone(), amount)
                    .await
            }
            InputType::LnUrlError { data } => Err(LightningError::SendLNURLPayment(data.reason)),
            _ => Err(LightningError::UnsupportedPaymentFormat(
                "Unsupported payment format".to_string(),
            )
            .into()),
        }?;

        payment.lightning_address = Some(ln_address.username.clone());
        payment.status = match payment.error {
            Some(_) => "FAILED".to_string(),
            None => "PENDING".to_string(),
        };
        payment.id = Uuid::new_v4();
        payment = self.store.payment_repo.insert(payment).await?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        info!(
            user_id = user.sub,
            input,
            payment_hash = payment.payment_hash,
            amount_msat,
            status = payment.status,
            "Payment processed successfully"
        );
        Ok(payment)
    }
}

pub fn validate_amount(amount_msat: Option<u64>) -> Result<u64, ApplicationError> {
    let amount = amount_msat
        .ok_or_else(|| DataError::Validation("amount_msat must be defined".to_string()))?;
    if amount <= 0 {
        return Err(DataError::Validation("amount_msat must be greater than 0".to_string()).into());
    }

    Ok(amount)
}

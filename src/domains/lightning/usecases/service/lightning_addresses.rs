use async_trait::async_trait;
use breez_sdk_core::{parse, InputType, LNInvoice, LnUrlPayRequestData};
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    application::errors::{ApplicationError, DataError, DatabaseError, LightningError},
    domains::{
        lightning::{
            entities::{
                LNURLPayRequest, LightningAddress, LightningInvoice, LightningInvoiceStatus,
                LightningPayment, LightningPaymentStatus, UserBalance,
            },
            usecases::LightningAddressesUseCases,
        },
        users::entities::{AuthUser, Permission},
    },
};

use super::LightningService;

impl LightningService {
    fn validate_amount(amount_msat: Option<u64>) -> Result<u64, ApplicationError> {
        let amount = amount_msat.unwrap_or_default();
        if amount == 0 {
            return Err(DataError::Validation("Amount must be greater than 0".to_string()).into());
        }

        Ok(amount)
    }

    async fn send_bolt11(
        &self,
        invoice: LNInvoice,
        username: String,
        amount_msat: Option<u64>,
    ) -> Result<LightningPayment, ApplicationError> {
        let specified_amount = invoice.amount_msat.or(amount_msat);
        if specified_amount == Some(0) {
            return Err(
                DataError::Validation("Amount must be greater than zero.".to_string()).into(),
            );
        }

        let txn = self.store.begin().await?;

        // TODO: Add UUID to labels
        let balance = self
            .store
            .get_balance_by_username(Some(&txn), &username)
            .await?;

        if let Some(amount) = specified_amount {
            if balance.available_msat < amount as i64 {
                return Err(DataError::InsufficientFunds.into());
            }

            let pending_payment = self
                .store
                .insert_payment(Some(&txn), Some(username), "PENDING".to_string(), amount)
                .await?;

            txn.commit()
                .await
                .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

            let result = self
                .lightning_client
                .send_payment(
                    invoice.bolt11.clone(),
                    if invoice.amount_msat.is_some() {
                        None
                    } else {
                        Some(amount)
                    },
                )
                .await;

            self.handle_processed_payment(pending_payment, result).await
        } else {
            Err(DataError::Validation(
                "Amount must be defined for zero-amount invoices.".to_string(),
            )
            .into())
        }
    }

    async fn send_lnurl_pay(
        &self,
        data: LnUrlPayRequestData,
        username: String,
        amount_msat: Option<u64>,
        comment: Option<String>,
    ) -> Result<LightningPayment, ApplicationError> {
        let amount = LightningService::validate_amount(amount_msat)?;

        let txn = self.store.begin().await?;

        // TODO: Add UUID to labels and only encapsulate the insert PENDING in a transaction
        let balance = self
            .store
            .get_balance_by_username(Some(&txn), &username)
            .await?;

        if balance.available_msat <= amount as i64 {
            return Err(DataError::InsufficientFunds.into());
        }

        let pending_payment = self
            .store
            .insert_payment(Some(&txn), Some(username), "PENDING".to_string(), amount)
            .await?;

        txn.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let result = self.lightning_client.lnurl_pay(data, amount, comment).await;

        self.handle_processed_payment(pending_payment, result).await
    }

    async fn handle_processed_payment(
        &self,
        mut pending_payment: LightningPayment,
        result: Result<LightningPayment, LightningError>,
    ) -> Result<LightningPayment, ApplicationError> {
        match result {
            Ok(mut payment) => {
                payment.id = pending_payment.id;
                payment.status = match payment.error {
                    // There is a case where the payment fails but still has a payment_hash and a payment is returned insteaf of an error
                    Some(_) => LightningPaymentStatus::FAILED,
                    None => LightningPaymentStatus::SETTLED,
                };
                let payment: LightningPayment = self.store.update_payment(payment).await?;
                Ok(payment)
            }
            Err(error) => {
                pending_payment.status = LightningPaymentStatus::FAILED;
                pending_payment.error = Some(error.to_string());
                let payment: LightningPayment = self.store.update_payment(pending_payment).await?;
                Ok(payment)
            }
        }
    }
}

#[async_trait]
impl LightningAddressesUseCases for LightningService {
    async fn generate_lnurlp(&self, username: String) -> Result<LNURLPayRequest, ApplicationError> {
        debug!(username, "Generating LNURLp");

        self.store
            .find_address_by_username(&username)
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
            .find_address_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        let mut invoice = self.lightning_client.invoice(amount, description).await?;
        invoice.lightning_address = Some(username.clone());
        invoice.status = LightningInvoiceStatus::PENDING;
        invoice.id = Uuid::new_v4();

        let invoice = self.store.insert_invoice(invoice).await?;

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
            .find_address_by_username(&user.sub)
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
            .find_address_by_username(&username)
            .await?
            .is_some()
        {
            return Err(DataError::Conflict("Username already exists.".to_string()).into());
        }

        let lightning_address = self.store.insert_address(&user.sub, &username).await?;

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
            .find_address_by_username(&username)
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
            self.store.find_all_addresses(limit, offset).await?
        } else {
            // The user can only view their own addresses
            self.store
                .find_all_addresses_by_user_id(&user.sub, limit, offset)
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
            .find_address_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        if lightning_address.user_id != user.sub {
            user.check_permission(Permission::ReadLightningAddress)?;
        }

        let balance = self.store.get_balance_by_username(None, &username).await?;

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
            .find_address_by_user_id(&user.sub)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        let input_type = parse(&input)
            .await
            .map_err(|e| DataError::Validation(e.to_string()))?;

        let payment = match input_type {
            InputType::Bolt11 { invoice } => {
                self.send_bolt11(invoice, ln_address.username, amount_msat)
                    .await
            }
            InputType::LnUrlPay { data } => {
                self.send_lnurl_pay(data, ln_address.username, amount_msat, comment)
                    .await
            }
            InputType::LnUrlError { data } => Err(DataError::Validation(data.reason).into()),
            _ => Err(DataError::Validation("Unsupported payment format".to_string()).into()),
        }?;

        info!(
            user_id = user.sub,
            input,
            amount_msat,
            status = payment.status.to_string(),
            "Payment processed successfully"
        );

        Ok(payment)
    }
}

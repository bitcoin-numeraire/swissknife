use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{
        invoice::{Invoice, LnInvoiceOptions},
        payment::{LnPaymentTarget, Payment},
        system::HealthStatus,
    },
};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LnClient: Sync + Send {
    async fn decode_bolt12(&self, _input: &str) -> Result<crate::domains::payment::DecodedBolt12, LightningError> {
        Err(LightningError::InvalidPaymentTarget(
            "This Lightning provider does not support BOLT12".into(),
        ))
    }
    async fn fetch_offer_invoice(
        &self,
        _offer: &str,
        _amount_msat: u64,
        _note: Option<String>,
    ) -> Result<String, LightningError> {
        Err(LightningError::InvalidPaymentTarget(
            "This Lightning provider does not support BOLT12".into(),
        ))
    }

    async fn disconnect(&self) -> Result<(), LightningError>;
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        label: String,
        expiry: u32,
        options: LnInvoiceOptions,
    ) -> Result<Invoice, LightningError>;
    /// Return a provider-derived route fee estimate.
    async fn estimate_fee(&self, target: LnPaymentTarget) -> Result<u64, LightningError>;
    fn fee_limit_msat(&self, amount_msat: u64) -> u64;
    async fn pay(
        &self,
        bolt11: String,
        amount_msat: Option<u64>,
        fee_limit_msat: u64,
        label: String,
    ) -> Result<Payment, LightningError>;
    async fn invoice_by_hash(&self, payment_hash: String) -> Result<Option<Invoice>, LightningError>;
    async fn payment_by_hash(&self, payment_hash: String) -> Result<Option<Payment>, LightningError>;
    async fn cancel_invoice(&self, payment_hash: String, label: String) -> Result<(), LightningError>;
    async fn health(&self) -> Result<HealthStatus, LightningError>;
}

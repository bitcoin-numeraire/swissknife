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
    fn offer_backend(&self) -> Option<String> {
        None
    }
    async fn issue_offer(
        &self,
        _offer: &swissknife_types::WalletOffer,
    ) -> Result<crate::domains::offer::NodeOffer, LightningError> {
        Err(LightningError::InvalidPaymentTarget("BOLT12 offers unavailable".into()))
    }
    async fn local_offers(&self, _id: Option<String>) -> Result<Vec<crate::domains::offer::NodeOffer>, LightningError> {
        Err(LightningError::InvalidPaymentTarget("BOLT12 offers unavailable".into()))
    }
    async fn disable_offer(&self, _id: &str) -> Result<(), LightningError> {
        Err(LightningError::InvalidPaymentTarget("BOLT12 offers unavailable".into()))
    }
    async fn offer_updates(&self, _start: u64) -> Result<Vec<crate::domains::offer::OfferUpdate>, LightningError> {
        Err(LightningError::InvalidPaymentTarget("BOLT12 offers unavailable".into()))
    }
    async fn decode_bolt12(&self, _input: &str) -> Result<crate::domains::payment::DecodedBolt12, LightningError> {
        Err(LightningError::InvalidPaymentTarget(
            "This Lightning provider does not support BOLT12".into(),
        ))
    }
    async fn bip353_instructions(
        &self,
        _name: &str,
    ) -> Result<crate::domains::payment::Bip353Instructions, LightningError> {
        Err(LightningError::InvalidPaymentTarget(
            "This Lightning provider does not support DNSSEC BIP353 resolution".into(),
        ))
    }
    async fn fetch_offer_invoice(
        &self,
        _offer: &str,
        _amount_msat: u64,
        _note: Option<String>,
        _bip353: Option<String>,
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

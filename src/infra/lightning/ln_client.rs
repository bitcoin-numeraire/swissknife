use async_trait::async_trait;

use crate::{
    application::errors::LightningError,
    domains::{
        invoice::Invoice,
        payment::{LnPaymentTarget, Payment},
        system::HealthStatus,
    },
};

pub enum PaymentLookup {
    Latest,
    Attempt(String),
    /// Only return this attempt when every group for the hash has failed.
    RetryCandidate(String),
}

impl PaymentLookup {
    pub fn matches_label(&self, label: Option<&str>) -> bool {
        match self {
            Self::Latest => true,
            Self::Attempt(expected) | Self::RetryCandidate(expected) => label == Some(expected),
        }
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LnClient: Sync + Send {
    async fn disconnect(&self) -> Result<(), LightningError>;
    async fn invoice(
        &self,
        amount_msat: u64,
        description: String,
        label: String,
        expiry: u32,
        deschashonly: bool,
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
    async fn payment_by_hash(
        &self,
        payment_hash: String,
        lookup: PaymentLookup,
    ) -> Result<Option<Payment>, LightningError>;
    async fn cancel_invoice(&self, payment_hash: String, label: String) -> Result<(), LightningError>;
    async fn health(&self) -> Result<HealthStatus, LightningError>;
}

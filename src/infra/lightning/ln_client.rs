use std::str::FromStr;

use async_trait::async_trait;
use lightning_invoice::Bolt11Invoice;

use crate::{
    application::errors::LightningError,
    domains::{invoice::Invoice, payment::Payment, system::HealthStatus},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LnFeeEstimate {
    pub estimated_fee_msat: u64,
}

pub(crate) struct LnPaymentTarget {
    pub destination: Vec<u8>,
    pub amount_msat: u64,
    pub final_cltv_delta: u32,
}

pub(crate) fn payment_target(bolt11: &str, amount_msat: Option<u64>) -> Result<LnPaymentTarget, LightningError> {
    let invoice =
        Bolt11Invoice::from_str(bolt11).map_err(|err| LightningError::Pay(format!("Invalid BOLT11 invoice: {err}")))?;
    let destination = invoice
        .payee_pub_key()
        .copied()
        .unwrap_or_else(|| invoice.recover_payee_pub_key())
        .serialize()
        .to_vec();
    let amount_msat = invoice
        .amount_milli_satoshis()
        .or(amount_msat)
        .filter(|amount| *amount > 0)
        .ok_or_else(|| LightningError::Pay("Amount is required for a zero-amount invoice".to_string()))?;
    let final_cltv_delta = u32::try_from(invoice.min_final_cltv_expiry_delta())
        .map_err(|_| LightningError::Pay("Invoice final CLTV delta is too large".to_string()))?;

    Ok(LnPaymentTarget {
        destination,
        amount_msat,
        final_cltv_delta,
    })
}

pub(crate) fn cln_fee_limit_msat(configured_maxfee: Option<u64>, amount_msat: u64) -> u64 {
    configured_maxfee.unwrap_or_else(|| amount_msat.div_ceil(100).max(5_000))
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
    async fn estimate_fee(&self, bolt11: String, amount_msat: Option<u64>) -> Result<LnFeeEstimate, LightningError>;
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

#[cfg(test)]
mod tests {
    use super::cln_fee_limit_msat;

    #[test]
    fn cln_fee_limit_mirrors_xpay_percentage_with_floor() {
        assert_eq!(cln_fee_limit_msat(None, 100_000), 5_000);
        assert_eq!(cln_fee_limit_msat(None, 1_000_000), 10_000);
    }

    #[test]
    fn cln_fee_limit_uses_explicit_operator_policy() {
        assert_eq!(cln_fee_limit_msat(Some(2_500), 1_000_000), 2_500);
    }
}

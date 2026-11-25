use chrono::Utc;
use serde::Deserialize;

use super::PaymentService;
use crate::{
    application::errors::{ApplicationError, DataError},
    domains::bitcoin::BtcNetwork,
};

/// CLN validates the BOLT12 signature and required TLVs. The application validates the payer's intent.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DecodedBolt12 {
    #[serde(rename = "type")]
    pub kind: String,
    pub valid: bool,
    pub offer_id: Option<String>,
    #[serde(default)]
    pub offer_chains: Vec<String>,
    pub offer_description: Option<String>,
    pub offer_amount_msat: Option<u64>,
    pub offer_currency: Option<String>,
    pub offer_quantity_max: Option<u64>,
    #[serde(default, deserialize_with = "present")]
    pub offer_recurrence: bool,
    pub offer_absolute_expiry: Option<u64>,
    pub invreq_chain: Option<String>,
    pub invoice_created_at: Option<u64>,
    pub invoice_relative_expiry: Option<u64>,
    pub invoice_amount_msat: Option<u64>,
    pub invoice_payment_hash: Option<String>,
}

fn present<'de, D: serde::Deserializer<'de>>(de: D) -> Result<bool, D::Error> {
    Ok(Option::<serde::de::IgnoredAny>::deserialize(de)?.is_some())
}

#[derive(Clone, Debug)]
pub(super) struct ResolvedBolt12 {
    pub invoice: String,
    pub hash: String,
    pub amount_msat: u64,
    pub description: Option<String>,
}

impl ResolvedBolt12 {
    pub(super) fn payment(&self, wallet_id: uuid::Uuid, comment: Option<String>) -> super::Payment {
        super::Payment {
            wallet_id,
            amount_msat: self.amount_msat,
            ledger: crate::application::composition::Ledger::Lightning,
            status: super::PaymentStatus::Pending,
            description: comment.or_else(|| self.description.clone()),
            lightning: Some(super::LnPayment {
                payment_hash: self.hash.clone(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

fn chain(network: BtcNetwork) -> Result<String, DataError> {
    let network = match network {
        BtcNetwork::Bitcoin => bitcoin::Network::Bitcoin,
        BtcNetwork::Testnet => bitcoin::Network::Testnet,
        BtcNetwork::Testnet4 => bitcoin::Network::Testnet4,
        BtcNetwork::Signet => bitcoin::Network::Signet,
        BtcNetwork::Regtest => bitcoin::Network::Regtest,
        BtcNetwork::Simnet => return Err(DataError::Validation("BOLT12 is unavailable on simnet".into())),
    };
    Ok(network.chain_hash().to_string())
}

impl DecodedBolt12 {
    fn validate(&self, network: BtcNetwork) -> Result<(), DataError> {
        if !self.valid {
            return Err(DataError::Validation("Invalid BOLT12 signature or fields".into()));
        }
        let current = chain(network)?;
        let mainnet = bitcoin::Network::Bitcoin.chain_hash().to_string();
        let chains_match = if self.offer_chains.is_empty() {
            current == mainnet
        } else {
            self.offer_chains.contains(&current)
        };
        if !chains_match
            || (self.kind == "bolt12 invoice" && self.invreq_chain.as_deref().unwrap_or(&mainnet) != current)
        {
            return Err(DataError::Validation(
                "BOLT12 request belongs to a different Bitcoin network".into(),
            ));
        }
        if self.offer_currency.is_some() || self.offer_quantity_max.is_some() || self.offer_recurrence {
            return Err(DataError::Validation(
                "Choose a single Bitcoin-denominated BOLT12 payment".into(),
            ));
        }
        if self.kind == "bolt12 invoice"
            && self
                .invoice_created_at
                .and_then(|start| start.checked_add(self.invoice_relative_expiry.unwrap_or(7200)))
                .is_none_or(|end| end <= Utc::now().timestamp() as u64)
        {
            return Err(DataError::Validation("BOLT12 invoice is expired".into()));
        }
        Ok(())
    }

    fn fixed_amount(&self, requested: Option<u64>) -> Result<u64, ApplicationError> {
        let fixed = if self.kind == "bolt12 invoice" {
            self.invoice_amount_msat
        } else {
            self.offer_amount_msat
        };
        if matches!((fixed,requested),(Some(a),Some(b)) if a!=b) {
            return Err(DataError::Validation("Approved amount differs from the BOLT12 request".into()).into());
        }
        PaymentService::validate_amount(fixed.or(requested))
    }
}

impl PaymentService {
    /// Inspecting an offer for a fee preview does not contact its issuer or create an invoice.
    pub(super) async fn inspect_bolt12(
        &self,
        wallet: uuid::Uuid,
        input: &str,
        requested: Option<u64>,
    ) -> Result<(DecodedBolt12, u64), ApplicationError> {
        let network = self.bitcoin_wallet.network();
        let decoded = self.ln_client.decode_bolt12(input).await?;
        decoded.validate(network)?;
        if !matches!(decoded.kind.as_str(), "bolt12 offer" | "bolt12 invoice") {
            return Err(DataError::Validation("Expected a BOLT12 offer or invoice".into()).into());
        }
        if decoded.kind == "bolt12 offer"
            && decoded
                .offer_absolute_expiry
                .is_some_and(|end| end <= Utc::now().timestamp() as u64)
        {
            return Err(DataError::Validation("BOLT12 offer is expired".into()).into());
        }
        if let Some(id) = decoded.offer_id.as_deref() {
            if self.store.offer.owner(id).await? == Some(wallet) {
                return Err(DataError::Validation("Cannot pay your own wallet offer".into()).into());
            }
        }
        let amount = decoded.fixed_amount(requested)?;
        Ok((decoded, amount))
    }

    pub(super) async fn resolve_bolt12(
        &self,
        wallet: uuid::Uuid,
        input: String,
        amount: Option<u64>,
        note: Option<&str>,
    ) -> Result<ResolvedBolt12, ApplicationError> {
        let (decoded, approved) = self.inspect_bolt12(wallet, &input, amount).await?;
        let (invoice, decoded) = if decoded.kind == "bolt12 offer" {
            let id = decoded
                .offer_id
                .ok_or_else(|| DataError::Validation("Missing BOLT12 offer ID".into()))?;
            let invoice = self
                .ln_client
                .fetch_offer_invoice(&input, approved, note.map(str::to_owned))
                .await?;
            let result = self.ln_client.decode_bolt12(&invoice).await?;
            result.validate(self.bitcoin_wallet.network())?;
            if result.offer_id.as_deref() != Some(id.as_str()) {
                return Err(DataError::Validation("Returned BOLT12 invoice is for another offer".into()).into());
            }
            (invoice, result)
        } else {
            (input, decoded)
        };
        if decoded.kind != "bolt12 invoice" || decoded.invoice_amount_msat != Some(approved) {
            return Err(
                DataError::Validation("Returned BOLT12 invoice differs from the approved amount".into()).into(),
            );
        }
        let hash = decoded
            .invoice_payment_hash
            .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| DataError::Validation("Missing BOLT12 payment hash".into()))?;
        Ok(ResolvedBolt12 {
            invoice,
            hash,
            amount_msat: approved,
            description: decoded.offer_description,
        })
    }
}

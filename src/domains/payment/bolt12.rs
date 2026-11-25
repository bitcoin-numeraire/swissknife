use chrono::Utc;
use serde::Deserialize;

use super::{payment_input::Bolt12Target, PaymentService};
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
    pub invreq_bip_353_name: Option<Bip353InvoiceName>,
    pub invoice_created_at: Option<u64>,
    pub invoice_relative_expiry: Option<u64>,
    pub invoice_amount_msat: Option<u64>,
    pub invoice_payment_hash: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Bip353InvoiceName {
    pub name: Option<String>,
    pub domain: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Bip353Instructions {
    pub proof: String,
    pub instructions: Vec<Bip353Instruction>,
}
#[derive(Debug, Deserialize)]
pub struct Bip353Instruction {
    pub offer: Option<String>,
    pub offchain_amount_msat: Option<u64>,
}

impl Bip353Instructions {
    pub(super) fn select(self, amount: Option<u64>) -> Result<(String, Option<u64>), DataError> {
        if self.proof.is_empty()
            || !self.proof.len().is_multiple_of(2)
            || !self.proof.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(DataError::Validation(
                "No validated DNSSEC proof was returned for this name".into(),
            ));
        }
        let mut offers = self
            .instructions
            .into_iter()
            .filter_map(|i| i.offer.map(|offer| (offer, i.offchain_amount_msat)));
        let (offer, fixed) = offers
            .next()
            .ok_or_else(|| DataError::Validation("This name has no Lightning offer".into()))?;
        if offers.next().is_some() || !offer.get(..4).is_some_and(|prefix| prefix.eq_ignore_ascii_case("lno1")) {
            return Err(DataError::Validation(
                "This name does not specify one unambiguous Lightning offer".into(),
            ));
        }
        if fixed.zip(amount).is_some_and(|(fixed, approved)| fixed != approved) {
            return Err(DataError::Validation(
                "Approved amount differs from the DNSSEC payment instructions".into(),
            ));
        }
        Ok((offer, fixed.or(amount)))
    }
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
    pub bip353: Option<String>,
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
                ln_address: self.bip353.as_ref().map(|name| format!("₿{name}")),
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
    pub(super) async fn expand_bolt12(
        &self,
        target: Bolt12Target,
        amount: Option<u64>,
    ) -> Result<(String, Option<u64>, Option<String>), ApplicationError> {
        match target {
            Bolt12Target::Encoded(input) => Ok((input, amount, None)),
            Bolt12Target::Name(name) => {
                let (offer, amount) = self.ln_client.bip353_instructions(&name).await?.select(amount)?;
                Ok((offer, amount, Some(name)))
            }
        }
    }
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
        target: Bolt12Target,
        amount: Option<u64>,
        note: Option<&str>,
        expected_id: Option<&str>,
    ) -> Result<ResolvedBolt12, ApplicationError> {
        let (input, amount, name) = self.expand_bolt12(target, amount).await?;
        let (decoded, approved) = self.inspect_bolt12(wallet, &input, amount).await?;
        if expected_id.is_some_and(|id| Some(id) != decoded.offer_id.as_deref()) {
            return Err(DataError::PaymentTargetChanged.into());
        }
        let (invoice, decoded) = if decoded.kind == "bolt12 offer" {
            let id = decoded
                .offer_id
                .ok_or_else(|| DataError::Validation("Missing BOLT12 offer ID".into()))?;
            let invoice = self
                .ln_client
                .fetch_offer_invoice(&input, approved, note.map(str::to_owned), name.clone())
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
        if let Some(name) = name.as_deref() {
            let signed = decoded
                .invreq_bip_353_name
                .as_ref()
                .and_then(|n| n.name.as_ref().zip(n.domain.as_ref()))
                .map(|(user, domain)| format!("{user}@{domain}"));
            if signed.as_deref() != Some(name) {
                return Err(
                    DataError::Validation("Signed invoice does not identify the approved BIP353 name".into()).into(),
                );
            }
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
            bip353: name,
        })
    }
}

#[cfg(test)]
mod bip353_tests {
    use super::super::payment_input::bip353_name;
    use super::*;

    #[test]
    fn validates_and_normalizes_dns_names() {
        assert_eq!(bip353_name("Shop@Example.TEST").unwrap(), "shop@example.test");
        assert!(bip353_name("xn--bcher-kva@example.test").is_ok());
        for bad in [
            "",
            "a@@example.test",
            "@example.test",
            "x@example..test",
            "u/extra@example.test",
            "ä@example.test",
            "a@evil.test/path",
        ] {
            assert!(bip353_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn enforces_one_proven_lightning_destination_and_signed_amount() {
        let read = |value: serde_json::Value| serde_json::from_value::<Bip353Instructions>(value).unwrap();
        use serde_json::json;
        let data = json!({"proof":"1122","instructions":[{"onchain":"tb1qexample"},{"offer":"lno1offer","offchain_amount_msat":47001}]});
        assert_eq!(
            read(data.clone()).select(None).unwrap(),
            ("lno1offer".into(), Some(47001))
        );
        assert!(read(data).select(Some(47000)).is_err());
        for input in [
            json!({"proof":"","instructions":[{"offer":"lno1offer"}]}),
            json!({"proof":"gg","instructions":[{"offer":"lno1offer"}]}),
            json!({"proof":"1122","instructions":[{"offer":"lno1a"},{"offer":"lno1b"}]}),
            json!({"proof":"1122","instructions":[{"onchain":"tb1qexample"}]}),
        ] {
            assert!(read(input).select(Some(1)).is_err());
        }
    }
}

use std::{borrow::Cow, str::FromStr};

use ::lnurl::{
    lightning_address::LightningAddress as LnurlLightningAddress, lnurl::LnUrl, AsyncClient as LnurlClient,
    LnUrlResponse,
};
use bip21::{
    de::{DeserializationError, DeserializationState, DeserializeParams, ParamKind},
    Param, Uri,
};
use bitcoin::{address::NetworkUnchecked, Address, Network as BitcoinNetwork};
use lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescriptionRef, Currency as Bolt11Currency, ParseOrSemanticError};
use reqwest::Url;
use serde_bolt::bitcoin::hashes::hex::ToHex;
use thiserror::Error;

use crate::{
    application::entities::Currency,
    domains::{bitcoin::BtcNetwork, lnurl::LnUrlPayRequestData},
};

#[derive(Debug)]
pub enum PaymentInput {
    BitcoinAddress(BitcoinAddressData),
    Bolt11(ParsedBolt11Invoice),
    LnUrlPay(LnUrlPayRequestData),
}

#[derive(Clone, Debug)]
pub struct BitcoinAddressData {
    pub address: String,
    pub amount_sat: Option<u64>,
    pub message: Option<String>,
    pub network: BtcNetwork,
}

#[derive(Clone, Debug)]
pub struct ParsedBolt11Invoice {
    pub bolt11: String,
    pub amount_msat: Option<u64>,
    pub payment_hash: String,
    pub description: Option<String>,
    pub currency: Currency,
}

#[derive(Clone, Debug, Default)]
struct PaymentUriExtras {
    lightning: Option<Bolt11Invoice>,
}

#[derive(Clone, Debug, Error)]
enum PaymentUriExtrasError {
    #[error("multiple lightning parameters")]
    MultipleLightning,
    #[error("invalid lightning invoice")]
    InvalidLightningInvoice,
}

impl From<ParseOrSemanticError> for PaymentUriExtrasError {
    fn from(_: ParseOrSemanticError) -> Self {
        Self::InvalidLightningInvoice
    }
}

impl DeserializationError for PaymentUriExtras {
    type Error = PaymentUriExtrasError;
}

impl<'de> DeserializeParams<'de> for PaymentUriExtras {
    type DeserializationState = PaymentUriExtras;
}

impl<'de> DeserializationState<'de> for PaymentUriExtras {
    type Value = PaymentUriExtras;

    fn is_param_known(&self, key: &str) -> bool {
        key == "lightning"
    }

    fn deserialize_temp(&mut self, key: &str, value: Param<'_>) -> Result<ParamKind, PaymentUriExtrasError> {
        match key {
            "lightning" if self.lightning.is_none() => {
                let invoice_str: Cow<'_, str> = value
                    .try_into()
                    .map_err(|_| PaymentUriExtrasError::InvalidLightningInvoice)?;
                self.lightning = Some(Bolt11Invoice::from_str(&invoice_str)?);
                Ok(ParamKind::Known)
            }
            "lightning" => Err(PaymentUriExtrasError::MultipleLightning),
            _ => Ok(ParamKind::Unknown),
        }
    }

    fn finalize(self) -> Result<Self::Value, PaymentUriExtrasError> {
        Ok(self)
    }
}

pub async fn parse_payment_input(input: &str) -> Result<PaymentInput, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Payment input cannot be empty".to_string());
    }

    if let Ok(invoice) = parse_bolt11(input) {
        return Ok(PaymentInput::Bolt11(invoice));
    }

    if let Ok(bitcoin_payment) = parse_bitcoin_payment_input(input) {
        return Ok(bitcoin_payment);
    }

    if looks_like_lnurl(input) {
        let data = resolve_lnurl_pay(input).await?;
        return Ok(PaymentInput::LnUrlPay(data));
    }

    Err("Unsupported payment input".to_string())
}

fn parse_bolt11(input: &str) -> Result<ParsedBolt11Invoice, String> {
    let normalized = strip_lightning_scheme(input);
    let invoice = Bolt11Invoice::from_str(normalized).map_err(|err| err.to_string())?;
    parsed_bolt11_from_invoice(normalized.to_string(), invoice)
}

fn parsed_bolt11_from_invoice(bolt11: String, invoice: Bolt11Invoice) -> Result<ParsedBolt11Invoice, String> {
    let description = match invoice.description() {
        Bolt11InvoiceDescriptionRef::Direct(description) => Some(description.to_string()),
        Bolt11InvoiceDescriptionRef::Hash(_) => None,
    };

    Ok(ParsedBolt11Invoice {
        bolt11,
        amount_msat: invoice.amount_milli_satoshis(),
        payment_hash: invoice.payment_hash().to_hex(),
        description,
        currency: currency_from_bolt11(invoice.currency()),
    })
}

fn parse_bitcoin_payment_input(input: &str) -> Result<PaymentInput, String> {
    let uri = if input.to_ascii_lowercase().starts_with("bitcoin:") {
        input.to_string()
    } else {
        format!("bitcoin:{input}")
    };

    let uri = uri
        .parse::<Uri<'_, NetworkUnchecked, PaymentUriExtras>>()
        .map_err(|err| err.to_string())?;

    if let Some(invoice) = uri.extras.lightning {
        return parsed_bolt11_from_invoice(invoice.to_string(), invoice).map(PaymentInput::Bolt11);
    }

    let amount_sat = uri.amount.map(|amount| amount.to_sat());
    let message = uri
        .message
        .map(|message| {
            let message: Cow<'_, str> = message
                .try_into()
                .map_err(|err| format!("Invalid Bitcoin URI message: {err}"))?;
            Ok::<String, String>(message.into_owned())
        })
        .transpose()?;

    bitcoin_address_data_from_unchecked(uri.address, amount_sat, message).map(PaymentInput::BitcoinAddress)
}

fn parse_bitcoin_address_value(address: &str) -> Result<BitcoinAddressData, String> {
    let unchecked = Address::from_str(address).map_err(|err| err.to_string())?;
    bitcoin_address_data_from_unchecked(unchecked, None, None)
}

fn bitcoin_address_data_from_unchecked(
    unchecked: Address<NetworkUnchecked>,
    amount_sat: Option<u64>,
    message: Option<String>,
) -> Result<BitcoinAddressData, String> {
    let candidates = [
        (BitcoinNetwork::Bitcoin, BtcNetwork::Bitcoin),
        (BitcoinNetwork::Testnet, BtcNetwork::Testnet),
        (BitcoinNetwork::Testnet4, BtcNetwork::Testnet4),
        (BitcoinNetwork::Signet, BtcNetwork::Signet),
        (BitcoinNetwork::Regtest, BtcNetwork::Regtest),
    ];

    for (bitcoin_network, network) in candidates {
        if let Ok(checked) = unchecked.clone().require_network(bitcoin_network) {
            return Ok(BitcoinAddressData {
                address: checked.to_string(),
                amount_sat,
                message,
                network,
            });
        }
    }

    Err("Unsupported Bitcoin address network".to_string())
}

fn looks_like_lnurl(input: &str) -> bool {
    let normalized = strip_lightning_scheme(input);
    let lower = normalized.to_ascii_lowercase();
    lower.starts_with("lnurl")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || LnurlLightningAddress::from_str(normalized).is_ok()
}

async fn resolve_lnurl_pay(input: &str) -> Result<LnUrlPayRequestData, String> {
    let (url, ln_address) = lnurl_endpoint(input)?;
    let client = LnurlClient::from_client(reqwest::Client::new());
    let response = client.make_request(&url).await.map_err(|err| err.to_string())?;

    let LnUrlResponse::LnUrlPayResponse(pay) = response else {
        return Err("Unsupported LNURL response type".to_string());
    };

    lnurl_pay_request_data_from_response(pay, ln_address)
}

fn lnurl_endpoint(input: &str) -> Result<(String, Option<String>), String> {
    let input = strip_lightning_scheme(input);

    if let Ok(address) = LnurlLightningAddress::from_str(input) {
        return Ok((address.lnurlp_url(), Some(address.to_string())));
    }

    if input.to_ascii_lowercase().starts_with("lnurl") {
        let lnurl = LnUrl::from_str(input).map_err(|err| err.to_string())?;
        validate_lnurl_endpoint(&lnurl.url)?;
        return Ok((lnurl.url, None));
    }

    validate_lnurl_endpoint(input)?;
    Ok((input.to_string(), None))
}

fn validate_lnurl_endpoint(endpoint: &str) -> Result<(), String> {
    let url = Url::parse(endpoint).map_err(|err| err.to_string())?;
    match url.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!("Unsupported LNURL scheme: {scheme}")),
    }
}

fn lnurl_pay_request_data_from_response(
    pay: ::lnurl::pay::PayResponse,
    ln_address: Option<String>,
) -> Result<LnUrlPayRequestData, String> {
    let comment_allowed = match pay.comment_allowed {
        Some(comment_allowed) => {
            u16::try_from(comment_allowed).map_err(|_| "LNURL commentAllowed exceeds supported range".to_string())?
        }
        None => 0,
    };

    Ok(LnUrlPayRequestData {
        callback: pay.callback,
        min_sendable: pay.min_sendable,
        max_sendable: pay.max_sendable,
        metadata: pay.metadata,
        tag: pay.tag.to_string(),
        comment_allowed,
        ln_address,
    })
}

fn strip_lightning_scheme(input: &str) -> &str {
    input
        .strip_prefix("lightning:")
        .or_else(|| input.strip_prefix("LIGHTNING:"))
        .unwrap_or(input)
}

fn currency_from_bolt11(currency: Bolt11Currency) -> Currency {
    match currency {
        Bolt11Currency::Bitcoin => Currency::Bitcoin,
        Bolt11Currency::BitcoinTestnet => Currency::BitcoinTestnet,
        Bolt11Currency::Regtest => Currency::Regtest,
        Bolt11Currency::Simnet => Currency::Simnet,
        Bolt11Currency::Signet => Currency::Signet,
    }
}

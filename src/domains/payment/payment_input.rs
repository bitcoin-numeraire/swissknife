use std::str::FromStr;

use bech32::FromBase32;
use bitcoin::{Address, Network as BitcoinNetwork};
use lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescriptionRef, Currency as Bolt11Currency};
use reqwest::Url;
use serde_bolt::bitcoin::hashes::hex::ToHex;

use crate::{
    application::entities::Currency,
    domains::{
        bitcoin::BtcNetwork,
        lnurl::{LnUrlErrorData, LnUrlPayRequestData},
    },
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

pub async fn parse_payment_input(input: &str) -> Result<PaymentInput, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Payment input cannot be empty".to_string());
    }

    if let Ok(invoice) = parse_bolt11(input) {
        return Ok(PaymentInput::Bolt11(invoice));
    }

    if let Ok(address) = parse_bitcoin_address(input) {
        return Ok(PaymentInput::BitcoinAddress(address));
    }

    if looks_like_lnurl(input) {
        let data = resolve_lnurl_pay(input).await?;
        return Ok(PaymentInput::LnUrlPay(data));
    }

    Err("Unsupported payment input".to_string())
}

fn parse_bolt11(input: &str) -> Result<ParsedBolt11Invoice, String> {
    let invoice = Bolt11Invoice::from_str(input).map_err(|err| err.to_string())?;
    let description = match invoice.description() {
        Bolt11InvoiceDescriptionRef::Direct(description) => Some(description.to_string()),
        Bolt11InvoiceDescriptionRef::Hash(_) => None,
    };

    Ok(ParsedBolt11Invoice {
        bolt11: input.to_string(),
        amount_msat: invoice.amount_milli_satoshis(),
        payment_hash: invoice.payment_hash().to_hex(),
        description,
        currency: currency_from_bolt11(invoice.currency()),
    })
}

fn parse_bitcoin_address(input: &str) -> Result<BitcoinAddressData, String> {
    if input.to_ascii_lowercase().starts_with("bitcoin:") {
        return parse_bip21(input);
    }

    parse_bitcoin_address_value(input, None, None)
}

fn parse_bip21(input: &str) -> Result<BitcoinAddressData, String> {
    let uri = Url::parse(input).map_err(|err| err.to_string())?;
    if !uri.scheme().eq_ignore_ascii_case("bitcoin") {
        return Err("Unsupported URI scheme".to_string());
    }

    let address = uri.host_str().unwrap_or_else(|| uri.path());
    let mut amount_sat = None;
    let mut message = None;

    for (key, value) in uri.query_pairs() {
        match key.as_ref() {
            "amount" => amount_sat = Some(parse_btc_amount_to_sat(value.as_ref())?),
            "message" => message = Some(value.to_string()),
            _ => {}
        }
    }

    parse_bitcoin_address_value(address, amount_sat, message)
}

fn parse_bitcoin_address_value(
    address: &str,
    amount_sat: Option<u64>,
    message: Option<String>,
) -> Result<BitcoinAddressData, String> {
    let unchecked = Address::from_str(address).map_err(|err| err.to_string())?;
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

fn parse_btc_amount_to_sat(amount: &str) -> Result<u64, String> {
    let (whole, fractional) = amount.split_once('.').unwrap_or((amount, ""));
    if fractional.len() > 8 {
        return Err("Bitcoin amount has more than 8 decimal places".to_string());
    }

    let whole_sat = whole
        .parse::<u64>()
        .map_err(|err| format!("Invalid Bitcoin amount: {err}"))?
        .checked_mul(100_000_000)
        .ok_or_else(|| "Bitcoin amount overflows satoshi range".to_string())?;

    let mut fractional = fractional.to_string();
    while fractional.len() < 8 {
        fractional.push('0');
    }

    let fractional_sat = if fractional.is_empty() {
        0
    } else {
        fractional
            .parse::<u64>()
            .map_err(|err| format!("Invalid Bitcoin amount: {err}"))?
    };

    whole_sat
        .checked_add(fractional_sat)
        .ok_or_else(|| "Bitcoin amount overflows satoshi range".to_string())
}

fn looks_like_lnurl(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    lower.starts_with("lnurl")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || is_lightning_address(input)
}

fn is_lightning_address(input: &str) -> bool {
    let Some((name, domain)) = input.split_once('@') else {
        return false;
    };

    !name.is_empty() && !domain.is_empty() && domain.contains('.')
}

async fn resolve_lnurl_pay(input: &str) -> Result<LnUrlPayRequestData, String> {
    let (url, ln_address) = lnurl_endpoint(input)?;
    let response = reqwest::get(url.clone()).await.map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("LNURL endpoint returned HTTP {}", response.status()));
    }

    let body = response.text().await.map_err(|err| err.to_string())?;
    if let Ok(error) = serde_json::from_str::<LnUrlErrorData>(&body) {
        if !error.reason.is_empty() {
            return Err(error.reason);
        }
    }

    let mut data: LnUrlPayRequestData = serde_json::from_str(&body).map_err(|err| err.to_string())?;
    if data.tag != "payRequest" {
        return Err(format!("Unsupported LNURL tag: {}", data.tag));
    }
    data.ln_address = ln_address;

    Ok(data)
}

fn lnurl_endpoint(input: &str) -> Result<(Url, Option<String>), String> {
    if is_lightning_address(input) {
        let (name, domain) = input.split_once('@').expect("checked above");
        let url = Url::parse(&format!("https://{domain}/.well-known/lnurlp/{name}")).map_err(|err| err.to_string())?;
        return Ok((url, Some(input.to_string())));
    }

    if input.to_ascii_lowercase().starts_with("lnurl") {
        let (hrp, data, _) = bech32::decode(input).map_err(|err| err.to_string())?;
        if !hrp.eq_ignore_ascii_case("lnurl") {
            return Err("Invalid LNURL bech32 prefix".to_string());
        }

        let bytes = Vec::<u8>::from_base32(&data).map_err(|err| err.to_string())?;
        let url = String::from_utf8(bytes).map_err(|err| err.to_string())?;
        let url = Url::parse(&url).map_err(|err| err.to_string())?;
        return Ok((url, None));
    }

    let url = Url::parse(input).map_err(|err| err.to_string())?;
    match url.scheme() {
        "http" | "https" => Ok((url, None)),
        scheme => Err(format!("Unsupported LNURL scheme: {scheme}")),
    }
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

use std::str::FromStr;

use aes::Aes256;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use lightning_invoice::Bolt11Invoice;
use reqwest::Url;
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tracing::{trace, warn};

use crate::domains::lnurl::{LnUrlErrorData, LnUrlPayCallbackResponse, LnUrlPayRequestData, LnUrlPaySuccessAction};

use super::LnUrlSuccessAction;

type Aes256CbcDec = cbc::Decryptor<Aes256>;

pub async fn validate_lnurl_pay(
    user_amount_msat: u64,
    comment: &Option<String>,
    req: &LnUrlPayRequestData,
) -> Result<LnUrlPayCallbackResponse> {
    trace!(?req, "Validating LNURL pay request");

    validate_user_input(user_amount_msat, comment, req)?;

    let amount_msat = user_amount_msat.to_string();
    let mut url = Url::from_str(&req.callback)?;

    url.query_pairs_mut().append_pair("amount", &amount_msat);
    if let Some(comment) = comment {
        url.query_pairs_mut().append_pair("comment", comment);
    }

    // TODO: Instantiate and reuse the client instead of using reqwest::get
    let response = reqwest::get(url).await?;
    let callback_resp_text = response.text().await?;

    if let Ok(err) = serde_json::from_str::<LnUrlErrorData>(&callback_resp_text) {
        return Err(anyhow!(err.reason));
    }

    let callback_resp: LnUrlPayCallbackResponse = serde_json::from_str(&callback_resp_text)?;
    if let Some(ref sa) = callback_resp.success_action {
        validate_success_action(sa)?;
    }

    validate_invoice(user_amount_msat, &callback_resp.pr)?;
    Ok(callback_resp)
}

fn validate_user_input(user_amount_msat: u64, comment: &Option<String>, req: &LnUrlPayRequestData) -> Result<()> {
    if user_amount_msat < req.min_sendable {
        return Err(anyhow!(format!(
            "Amount is smaller than the minimum allowed: {} sats",
            req.min_sendable_sats()
        )));
    }

    if user_amount_msat > req.max_sendable {
        return Err(anyhow!(format!(
            "Amount is bigger than the maximum allowed: {} sats",
            req.max_sendable_sats()
        )));
    }

    match comment {
        None => Ok(()),
        Some(msg) => match msg.len() <= req.comment_allowed as usize {
            true => Ok(()),
            false => Err(anyhow!(format!(
                "Comment is longer than the maximum allowed length: {}",
                req.comment_allowed
            ))),
        },
    }
}

fn validate_success_action(success_action: &LnUrlPaySuccessAction) -> Result<()> {
    match success_action {
        LnUrlPaySuccessAction::Message { message } => {
            if message.is_empty() {
                return Err(anyhow!("LNURL success action message cannot be empty"));
            }
        }
        LnUrlPaySuccessAction::Url { description, url } => {
            if description.is_empty() {
                return Err(anyhow!("LNURL success action URL description cannot be empty"));
            }
            Url::parse(url)?;
        }
        LnUrlPaySuccessAction::Aes {
            description,
            ciphertext,
            iv,
        } => {
            if description.is_empty() {
                return Err(anyhow!("LNURL success action AES description cannot be empty"));
            }
            STANDARD.decode(ciphertext)?;
            let iv = STANDARD.decode(iv)?;
            if iv.len() != 16 {
                return Err(anyhow!("LNURL success action AES IV must be 16 bytes"));
            }
        }
    }

    Ok(())
}

fn validate_invoice(user_amount_msat: u64, bolt11: &str) -> Result<()> {
    let invoice = Bolt11Invoice::from_str(bolt11).map_err(|e| anyhow!(e.to_string()))?;

    match invoice.amount_milli_satoshis() {
        None => Err(anyhow!("Missing amount from invoice".to_string(),)),
        Some(invoice_amount_msat) => match invoice_amount_msat == user_amount_msat {
            true => Ok(()),
            false => Err(anyhow!(format!(
                "Invoice amount is different than user amount: {}",
                user_amount_msat
            ))),
        },
    }
}

pub fn process_success_action(sa: LnUrlPaySuccessAction, payment_preimage: &str) -> Option<LnUrlSuccessAction> {
    match sa {
        LnUrlPaySuccessAction::Aes {
            description,
            ciphertext,
            iv,
        } => decrypt_success_action(&ciphertext, &iv, payment_preimage).map(|plaintext| LnUrlSuccessAction {
            tag: "message".to_string(),
            message: Some(plaintext),
            description: Some(description),
            ..Default::default()
        }),
        LnUrlPaySuccessAction::Message { message } => Some(LnUrlSuccessAction {
            tag: "message".to_string(),
            message: Some(message),
            ..Default::default()
        }),
        LnUrlPaySuccessAction::Url { description, url } => Some(LnUrlSuccessAction {
            tag: "url".to_string(),
            description: Some(description),
            url: Some(url),
            ..Default::default()
        }),
    }
}

fn decrypt_success_action(ciphertext: &str, iv: &str, payment_preimage: &str) -> Option<String> {
    let preimage = match sha256::Hash::from_str(payment_preimage) {
        Ok(preimage) => preimage,
        Err(err) => {
            warn!(%err, payment_preimage, "Invalid payment preimage");
            return None;
        }
    };
    let preimage_arr: [u8; 32] = preimage.into_inner();

    let ciphertext = match STANDARD.decode(ciphertext) {
        Ok(ciphertext) => ciphertext,
        Err(err) => {
            warn!(%err, "Failed to decode LNURL success action AES ciphertext");
            return None;
        }
    };
    let iv = match STANDARD.decode(iv) {
        Ok(iv) => iv,
        Err(err) => {
            warn!(%err, "Failed to decode LNURL success action AES IV");
            return None;
        }
    };

    let plaintext = match Aes256CbcDec::new_from_slices(&preimage_arr, &iv)
        .ok()?
        .decrypt_padded_vec_mut::<Pkcs7>(&ciphertext)
    {
        Ok(plaintext) => plaintext,
        Err(err) => {
            warn!(%err, payment_preimage, "Failed to decrypt success action AES data");
            return None;
        }
    };

    match String::from_utf8(plaintext) {
        Ok(plaintext) => Some(plaintext),
        Err(err) => {
            warn!(%err, "LNURL success action AES plaintext is not UTF-8");
            None
        }
    }
}

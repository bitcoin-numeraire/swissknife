use std::str::FromStr;

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use anyhow::{anyhow, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use breez_sdk_spark::{LnurlPayRequestDetails, SuccessAction};
use lightning_invoice::Bolt11Invoice;
use reqwest::Url;
use serde::Deserialize;
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tracing::{trace, warn};

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

use crate::domains::lnurl::LnUrlErrorData;

use super::LnUrlSuccessAction;

/// Local struct matching the LNURL-pay callback JSON response.
/// The Spark SDK's `CallbackResponse` is internal and not re-exported.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CallbackResponse {
    pub pr: String,
    pub success_action: Option<SuccessAction>,
}

pub async fn validate_lnurl_pay(
    user_amount_msat: u64,
    comment: &Option<String>,
    req: &LnurlPayRequestDetails,
) -> Result<CallbackResponse> {
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

    let callback_resp: CallbackResponse = serde_json::from_str(&callback_resp_text)?;
    if let Some(ref sa) = callback_resp.success_action {
        validate_success_action(sa)?;
    }

    validate_invoice(user_amount_msat, &callback_resp.pr)?;
    Ok(callback_resp)
}

fn validate_success_action(sa: &SuccessAction) -> Result<()> {
    match sa {
        SuccessAction::Aes { data } => {
            if data.description.len() > 144 {
                return Err(anyhow!(
                    "AES action description length is larger than the maximum allowed"
                ));
            }
            if data.ciphertext.len() > 4096 {
                return Err(anyhow!(
                    "AES action ciphertext length is larger than the maximum allowed"
                ));
            }
            if data.iv.len() != 24 {
                return Err(anyhow!("AES action IV must be exactly 24 characters"));
            }
            Ok(())
        }
        SuccessAction::Message { data } => {
            if data.message.len() > 144 {
                return Err(anyhow!(
                    "Success action message is longer than the maximum allowed length"
                ));
            }
            Ok(())
        }
        SuccessAction::Url { data } => {
            if data.description.len() > 144 {
                return Err(anyhow!(
                    "Success action description is longer than the maximum allowed length"
                ));
            }
            Ok(())
        }
    }
}

fn validate_user_input(user_amount_msat: u64, comment: &Option<String>, req: &LnurlPayRequestDetails) -> Result<()> {
    if user_amount_msat < req.min_sendable {
        return Err(anyhow!(format!(
            "Amount is smaller than the minimum allowed: {} sats",
            req.min_sendable / 1000
        )));
    }

    if user_amount_msat > req.max_sendable {
        return Err(anyhow!(format!(
            "Amount is bigger than the maximum allowed: {} sats",
            req.max_sendable / 1000
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

pub fn process_success_action(sa: SuccessAction, payment_preimage: &str) -> Option<LnUrlSuccessAction> {
    match sa {
        // For AES, we decrypt the contents on the fly
        SuccessAction::Aes { data } => {
            let preimage = sha256::Hash::from_str(payment_preimage);

            match preimage {
                Ok(preimage) => {
                    let preimage_arr: [u8; 32] = preimage.into_inner();

                    let plaintext = match decrypt_aes_success_action(&preimage_arr, &data.iv, &data.ciphertext) {
                        Ok(plaintext) => plaintext,
                        Err(err) => {
                            warn!(%err, payment_preimage, "Failed to decrypt success action AES data");
                            return None;
                        }
                    };

                    // See https://github.com/lnurl/luds/blob/luds/10.md. Decrypted AES is to be displayed like a message
                    Some(LnUrlSuccessAction {
                        tag: "message".to_string(),
                        message: Some(plaintext),
                        description: Some(data.description),
                        ..Default::default()
                    })
                }
                Err(err) => {
                    warn!(%err, payment_preimage, "Invalid payment preimage");
                    None
                }
            }
        }
        SuccessAction::Message { data } => Some(LnUrlSuccessAction {
            tag: "message".to_string(),
            message: Some(data.message),
            ..Default::default()
        }),
        SuccessAction::Url { data } => Some(LnUrlSuccessAction {
            tag: "url".to_string(),
            description: Some(data.description),
            url: Some(data.url),
            ..Default::default()
        }),
    }
}

/// AES-256-CBC decrypt for LNURL success action (LUD-10).
fn decrypt_aes_success_action(key: &[u8; 32], iv_b64: &str, ciphertext_b64: &str) -> Result<String> {
    let iv = BASE64_STANDARD.decode(iv_b64)?;
    let ciphertext = BASE64_STANDARD.decode(ciphertext_b64)?;
    let plaintext = Aes256CbcDec::new_from_slices(key, &iv)?.decrypt_padded_vec_mut::<Pkcs7>(&ciphertext)?;
    Ok(String::from_utf8(plaintext)?)
}

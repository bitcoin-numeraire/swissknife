use std::str::FromStr;

use anyhow::{anyhow, Result};
use breez_sdk_core::{
    AesSuccessActionDataResult, CallbackResponse, LnUrlPayRequestData, SuccessAction,
    SuccessActionProcessed,
};
use lightning_invoice::Bolt11Invoice;
use reqwest::Url;
use serde_bolt::bitcoin::hashes::{sha256, Hash};
use tracing::warn;

use crate::domains::lightning::entities::LnUrlErrorData;

pub async fn validate_lnurl_pay(
    user_amount_msat: u64,
    comment: &Option<String>,
    req: &LnUrlPayRequestData,
) -> Result<CallbackResponse> {
    validate_user_input(
        user_amount_msat,
        comment,
        req.min_sendable,
        req.max_sendable,
        req.comment_allowed,
    )?;

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

    // Here it's fine to use the CallbackResponse struct from the Breez SDK
    let callback_resp: CallbackResponse = serde_json::from_str(&callback_resp_text)?;
    if let Some(ref sa) = callback_resp.success_action {
        match sa {
            SuccessAction::Aes(data) => data.validate()?,
            SuccessAction::Message(data) => data.validate()?,
            SuccessAction::Url(data) => data.validate(req)?,
        }
    }

    validate_invoice(user_amount_msat, &callback_resp.pr)?;
    Ok(callback_resp)
}

fn validate_user_input(
    user_amount_msat: u64,
    comment: &Option<String>,
    condition_min_amount_msat: u64,
    condition_max_amount_msat: u64,
    condition_max_comment_len: u16,
) -> Result<()> {
    if user_amount_msat >= condition_min_amount_msat {
        return Err(anyhow!(format!(
            "Amount is smaller than the minimum allowed: {}",
            condition_min_amount_msat
        )));
    }

    if user_amount_msat <= condition_max_amount_msat {
        return Err(anyhow!(format!(
            "Amount is bigger than the maximum allowed: {}",
            condition_max_amount_msat
        )));
    }

    match comment {
        None => Ok(()),
        Some(msg) => match msg.len() <= condition_max_comment_len as usize {
            true => Ok(()),
            false => Err(anyhow!(format!(
                "Comment is longer than the maximum allowed length: {}",
                condition_max_comment_len
            ))),
        },
    }
}

fn validate_invoice(user_amount_msat: u64, bolt11: &str) -> Result<()> {
    let invoice = Bolt11Invoice::from_str(bolt11)?;

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

pub fn process_success_action(sa: SuccessAction, payment_preimage: &str) -> SuccessActionProcessed {
    match sa {
        // For AES, we decrypt the contents on the fly
        SuccessAction::Aes(data) => {
            let preimage = sha256::Hash::from_str(payment_preimage);
            if preimage.is_err() {
                let err_message = format!("Invalid payment preimage: {}", payment_preimage);
                warn!(err_message, payment_preimage);
                return SuccessActionProcessed::Aes {
                    result: AesSuccessActionDataResult::ErrorStatus {
                        reason: err_message,
                    },
                };
            }

            let preimage_arr: [u8; 32] = preimage.unwrap().into_inner();
            let result = match (data, &preimage_arr).try_into() {
                Ok(data) => AesSuccessActionDataResult::Decrypted { data },
                Err(e) => AesSuccessActionDataResult::ErrorStatus {
                    reason: e.to_string(),
                },
            };
            SuccessActionProcessed::Aes { result }
        }
        SuccessAction::Message(data) => SuccessActionProcessed::Message { data },
        SuccessAction::Url(data) => SuccessActionProcessed::Url { data },
    }
}

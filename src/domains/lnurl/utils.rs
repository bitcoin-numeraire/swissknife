use std::str::FromStr;

use ::lnurl::{
    pay::{
        AesParams as LnurlAesParams, LnURLPayInvoice, PayResponse as LnurlPayResponse,
        SuccessAction as LnurlSuccessAction,
    },
    Tag as LnurlTag,
};
use anyhow::{anyhow, Result};
use lightning_invoice::Bolt11Invoice;
use reqwest::Url;
use serde::Deserialize;
use serde_bolt::bitcoin::hashes::sha256;
use tracing::{trace, warn};

use crate::domains::lnurl::{LnUrlPayCallbackResponse, LnUrlPayRequestData, LnUrlPaySuccessAction};

use super::LnUrlSuccessAction;

pub async fn validate_lnurl_pay(
    user_amount_msat: u64,
    comment: &Option<String>,
    req: &LnUrlPayRequestData,
) -> Result<LnUrlPayCallbackResponse> {
    trace!(?req, "Validating LNURL pay request");

    let pay = lnurl_pay_response_from_request(req);
    let callback_resp =
        fetch_lnurl_pay_invoice(reqwest::Client::new(), &pay, user_amount_msat, comment.as_deref()).await?;

    validate_invoice(user_amount_msat, &callback_resp.pr)?;

    let success_action = callback_resp
        .success_action()
        .map(|success_action| convert_success_action(success_action, &req.callback))
        .transpose()?;

    Ok(LnUrlPayCallbackResponse {
        pr: callback_resp.pr,
        success_action,
        disposable: None,
        routes: None,
    })
}

fn lnurl_pay_response_from_request(req: &LnUrlPayRequestData) -> LnurlPayResponse {
    LnurlPayResponse {
        callback: req.callback.clone(),
        max_sendable: req.max_sendable,
        min_sendable: req.min_sendable,
        tag: LnurlTag::PayRequest,
        metadata: req.metadata.clone(),
        comment_allowed: Some(req.comment_allowed.into()),
        allows_nostr: None,
        nostr_pubkey: None,
    }
}

async fn fetch_lnurl_pay_invoice(
    client: reqwest::Client,
    pay: &LnurlPayResponse,
    msats: u64,
    comment: Option<&str>,
) -> Result<LnURLPayInvoice> {
    let url = lnurl_pay_callback_url(pay, msats, comment)?;
    let response_text = client.get(url).send().await?.error_for_status()?.text().await?;
    parse_lnurl_pay_invoice_response(&response_text)
}

fn lnurl_pay_callback_url(pay: &LnurlPayResponse, msats: u64, comment: Option<&str>) -> Result<Url> {
    if msats < pay.min_sendable || msats > pay.max_sendable {
        return Err(anyhow!("Invalid LNURL payment amount"));
    }

    if let (Some(comment), Some(max_length)) = (comment, pay.comment_allowed) {
        if comment.len() > max_length as usize {
            return Err(anyhow!("Invalid LNURL comment"));
        }
    }

    let amount = msats.to_string();
    let mut url = Url::parse(&pay.callback)?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("amount", &amount);
        if let Some(comment) = comment {
            query.append_pair("comment", comment);
        }
    }

    Ok(url)
}

#[derive(Deserialize)]
struct LnUrlCallbackErrorResponse {
    status: String,
    reason: String,
}

fn parse_lnurl_pay_invoice_response(response_text: &str) -> Result<LnURLPayInvoice> {
    match serde_json::from_str::<LnUrlCallbackErrorResponse>(response_text) {
        Ok(err) if err.status.eq_ignore_ascii_case("ERROR") => return Err(anyhow!(err.reason)),
        _ => {}
    }

    Ok(serde_json::from_str(response_text)?)
}

fn validate_invoice(user_amount_msat: u64, bolt11: &str) -> Result<()> {
    let invoice = Bolt11Invoice::from_str(bolt11).map_err(|e| anyhow!(e.to_string()))?;

    match invoice.amount_milli_satoshis() {
        None => Err(anyhow!("Missing amount from invoice".to_string())),
        Some(invoice_amount_msat) => match invoice_amount_msat == user_amount_msat {
            true => Ok(()),
            false => Err(anyhow!(format!(
                "Invoice amount is different than user amount: {}",
                user_amount_msat
            ))),
        },
    }
}

fn convert_success_action(success_action: LnurlSuccessAction, callback_url: &str) -> Result<LnUrlPaySuccessAction> {
    match success_action {
        LnurlSuccessAction::Message(message) => {
            validate_lud09_text_len("Success action message", &message)?;
            Ok(LnUrlPaySuccessAction::Message { message })
        }
        LnurlSuccessAction::Url { url, description } => {
            validate_lud09_text_len("Success action description", &description)?;
            validate_success_action_url(callback_url, &url)?;
            Ok(LnUrlPaySuccessAction::Url {
                description,
                url: url.to_string(),
            })
        }
        LnurlSuccessAction::AES(params) => {
            validate_aes_success_action(&params)?;
            Ok(LnUrlPaySuccessAction::Aes {
                description: params.description,
                ciphertext: params.ciphertext,
                iv: params.iv,
            })
        }
        LnurlSuccessAction::Unknown(params) => {
            Err(anyhow!(format!("Unsupported LNURL success action tag: {}", params.tag)))
        }
    }
}

fn validate_lud09_text_len(field: &str, value: &str) -> Result<()> {
    if value.len() > 144 {
        return Err(anyhow!(format!("{field} is longer than the maximum allowed length")));
    }

    Ok(())
}

fn validate_aes_success_action(params: &LnurlAesParams) -> Result<()> {
    validate_lud09_text_len("AES action description", &params.description)?;
    if params.ciphertext.len() > 4096 {
        return Err(anyhow!(
            "AES action ciphertext is longer than the maximum allowed length"
        ));
    }
    if params.iv.len() != 24 {
        return Err(anyhow!("AES action IV has unexpected length"));
    }

    // `lnurl-rs` performs the base64/AES decode during `decrypt`. We keep the
    // length checks above here because they are LUD-09/LUD-10 input guards.
    Ok(())
}

fn validate_success_action_url(callback_url: &str, action_url: &Url) -> Result<()> {
    let callback = Url::parse(callback_url)?;
    let callback_domain = callback
        .domain()
        .ok_or_else(|| anyhow!("Could not determine LNURL callback domain"))?;
    let action_domain = action_url
        .domain()
        .ok_or_else(|| anyhow!("Could not determine success action URL domain"))?;

    if callback_domain != action_domain {
        return Err(anyhow!(
            "Success action URL has different domain than the callback domain"
        ));
    }

    Ok(())
}

pub fn process_success_action(sa: LnUrlPaySuccessAction, payment_preimage: &str) -> Option<LnUrlSuccessAction> {
    match sa {
        LnUrlPaySuccessAction::Aes {
            description,
            ciphertext,
            iv,
        } => decrypt_success_action(ciphertext, iv, payment_preimage).map(|plaintext| LnUrlSuccessAction {
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

fn decrypt_success_action(ciphertext: String, iv: String, payment_preimage: &str) -> Option<String> {
    let preimage = match sha256::Hash::from_str(payment_preimage) {
        Ok(preimage) => preimage,
        Err(err) => {
            warn!(%err, payment_preimage, "Invalid payment preimage");
            return None;
        }
    };
    let preimage_arr: [u8; 32] = preimage.to_byte_array();

    let aes_params = LnurlAesParams {
        description: String::new(),
        ciphertext,
        iv,
    };

    match aes_params.decrypt(&preimage_arr) {
        Ok(plaintext) => Some(plaintext),
        Err(err) => {
            warn!(%err, payment_preimage, "Failed to decrypt LNURL success action AES data");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pay_response(callback: &str) -> LnurlPayResponse {
        LnurlPayResponse {
            callback: callback.to_string(),
            max_sendable: 10_000,
            min_sendable: 1_000,
            tag: LnurlTag::PayRequest,
            metadata: "[]".to_string(),
            comment_allowed: Some(144),
            allows_nostr: None,
            nostr_pubkey: None,
        }
    }

    #[test]
    fn lnurl_pay_callback_url_percent_encodes_comment() {
        let url = lnurl_pay_callback_url(
            &pay_response("https://example.com/lnurl/callback?existing=1"),
            2_000,
            Some("thanks & sats + #freedom"),
        )
        .unwrap();

        let query_pairs = url.query_pairs().into_owned().collect::<Vec<_>>();
        assert_eq!(
            query_pairs,
            vec![
                ("existing".to_string(), "1".to_string()),
                ("amount".to_string(), "2000".to_string()),
                ("comment".to_string(), "thanks & sats + #freedom".to_string()),
            ]
        );
        assert!(url.as_str().contains("comment=thanks+%26+sats+%2B+%23freedom"));
    }

    #[test]
    fn lnurl_pay_callback_url_rejects_invalid_amount() {
        let err = lnurl_pay_callback_url(&pay_response("https://example.com/callback"), 999, None).unwrap_err();
        assert!(err.to_string().contains("Invalid LNURL payment amount"));
    }

    #[test]
    fn lnurl_pay_callback_url_rejects_too_long_comment() {
        let mut pay = pay_response("https://example.com/callback");
        pay.comment_allowed = Some(3);

        let err = lnurl_pay_callback_url(&pay, 2_000, Some("four")).unwrap_err();
        assert!(err.to_string().contains("Invalid LNURL comment"));
    }

    #[test]
    fn parse_lnurl_pay_invoice_response_preserves_lnurl_error_reason() {
        let err = parse_lnurl_pay_invoice_response(r#"{"status":"ERROR","reason":"bad comment"}"#).unwrap_err();
        assert!(err.to_string().contains("bad comment"));
    }
}

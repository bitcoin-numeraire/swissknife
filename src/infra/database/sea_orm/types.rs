use std::time::Duration;

use chrono::Utc;

use crate::domains::{
    bitcoin::{BitcoinAddress, BitcoinOutput},
    invoice::{Invoice, InvoiceStatus, LnInvoice},
    ln_address::LnAddress,
    payment::Payment,
    user::ApiKey,
    wallet::{Contact, Wallet},
};

use super::models::{
    api_key::Model as ApiKeyModel, btc_address::Model as BitcoinAddressModel, btc_output::Model as BitcoinOutputModel,
    contact::ContactModel, invoice::Model as InvoiceModel, ln_address::Model as LnAddressModel,
    payment::Model as PaymentModel, wallet::Model as WalletModel,
};

const ASSERTION_MSG: &str = "should parse successfully by assertion";

impl From<InvoiceModel> for Invoice {
    fn from(model: InvoiceModel) -> Self {
        let status = match model.payment_time {
            Some(_) => InvoiceStatus::Settled,
            None => match model.expires_at {
                Some(expires_at) if Utc::now() > expires_at.and_utc() => InvoiceStatus::Expired,
                _ => InvoiceStatus::Pending,
            },
        };

        let ln_invoice = match model.ledger.as_str() {
            "Lightning" => Some(LnInvoice {
                payment_hash: model.payment_hash.expect(ASSERTION_MSG),
                bolt11: model.bolt11.expect(ASSERTION_MSG),
                description_hash: model.description_hash,
                payee_pubkey: model.payee_pubkey.expect(ASSERTION_MSG),
                min_final_cltv_expiry_delta: model.min_final_cltv_expiry_delta.expect(ASSERTION_MSG) as u64,
                payment_secret: model.payment_secret.expect(ASSERTION_MSG),
                expiry: Duration::from_secs(model.expiry.expect(ASSERTION_MSG) as u64),
                expires_at: model.expires_at.expect(ASSERTION_MSG).and_utc(),
            }),
            _ => None,
        };

        Invoice {
            id: model.id,
            wallet_id: model.wallet_id,
            ln_address_id: model.ln_address_id,
            description: model.description,
            amount_msat: model.amount_msat.map(|v| v as u64),
            amount_received_msat: model.amount_received_msat.map(|v| v as u64),
            timestamp: model.timestamp.and_utc(),
            ledger: model.ledger.parse().expect(ASSERTION_MSG),
            currency: model.currency.parse().expect(ASSERTION_MSG),
            status,
            fee_msat: None,
            payment_time: model.payment_time.map(|t| t.and_utc()),
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
            ln_invoice,
            btc_output_id: model.btc_output_id,
            bitcoin_output: None,
        }
    }
}

impl From<PaymentModel> for Payment {
    fn from(model: PaymentModel) -> Self {
        Payment {
            id: model.id,
            wallet_id: model.wallet_id,
            ln_address: model.ln_address,
            payment_hash: model.payment_hash,
            payment_preimage: model.payment_preimage,
            error: model.error,
            amount_msat: model.amount_msat as u64,
            fee_msat: model.fee_msat.map(|v| v as u64),
            payment_time: model.payment_time.map(|t| t.and_utc()),
            status: model.status.parse().expect(ASSERTION_MSG),
            ledger: model.ledger.parse().expect(ASSERTION_MSG),
            currency: model.currency.parse().expect(ASSERTION_MSG),
            description: model.description,
            metadata: model.metadata,
            success_action: serde_json::from_value(model.success_action.unwrap_or_default()).ok(),
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
            btc_output_id: model.btc_output_id,
            destination_address: model.destination_address,
            bitcoin_output: None,
        }
    }
}

impl From<ContactModel> for Contact {
    fn from(model: ContactModel) -> Self {
        Contact {
            ln_address: model.ln_address,
            contact_since: model.contact_since.map(|t| t.and_utc()).unwrap(),
        }
    }
}

impl From<LnAddressModel> for LnAddress {
    fn from(model: LnAddressModel) -> Self {
        LnAddress {
            id: model.id,
            wallet_id: model.wallet_id,
            username: model.username,
            active: model.active,
            allows_nostr: model.allows_nostr,
            nostr_pubkey: model.nostr_pubkey.map(|k| k.parse().expect(ASSERTION_MSG)),
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
        }
    }
}

impl From<WalletModel> for Wallet {
    fn from(model: WalletModel) -> Self {
        Wallet {
            id: model.id,
            user_id: model.user_id,
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
            ..Default::default()
        }
    }
}

impl From<ApiKeyModel> for ApiKey {
    fn from(model: ApiKeyModel) -> Self {
        ApiKey {
            id: model.id,
            user_id: model.user_id,
            name: model.name,
            key: None,
            key_hash: model.key_hash,
            permissions: serde_json::from_value(model.permissions).expect(ASSERTION_MSG),
            description: model.description,
            created_at: model.created_at.and_utc(),
            expires_at: model.expires_at.map(|t| t.and_utc()),
        }
    }
}

impl From<BitcoinOutputModel> for BitcoinOutput {
    fn from(model: BitcoinOutputModel) -> Self {
        BitcoinOutput {
            id: model.id,
            outpoint: model.outpoint,
            txid: model.txid,
            output_index: model.output_index as u32,
            address: None,
            amount_sat: model.amount_sat,
            status: model.status.parse().expect(ASSERTION_MSG),
            timestamp: model.timestamp.map(|t| t.and_utc()),
            network: model.network.parse().expect(ASSERTION_MSG),
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
        }
    }
}

impl From<BitcoinAddressModel> for BitcoinAddress {
    fn from(model: BitcoinAddressModel) -> Self {
        BitcoinAddress {
            id: model.id,
            wallet_id: model.wallet_id,
            address: model.address,
            address_type: model.address_type.parse().expect(ASSERTION_MSG),
            used: model.used,
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
        }
    }
}

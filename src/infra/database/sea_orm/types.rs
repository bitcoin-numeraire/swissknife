use std::time::Duration;

use chrono::Utc;

use crate::{
    application::composition::Ledger,
    domains::{
        account::{Account, AccountPreferences, ApiKey, AuthIdentity},
        asset::Asset,
        bitcoin::{BtcAddress, BtcOutput},
        invoice::{Invoice, InvoiceStatus, LnInvoice},
        ln_address::LnAddress,
        payment::{BtcPayment, InternalPayment, LnPayment, Payment},
        wallet::{Balance, Contact, Wallet},
    },
};

use sea_orm::Order;
use swissknife_types::OrderDirection;

use super::models::{
    account::Model as AccountModel, account_preference::Model as AccountPreferenceModel, api_key::Model as ApiKeyModel,
    asset::Model as AssetModel, auth_identity::Model as AuthIdentityModel, btc_address::Model as BitcoinAddressModel,
    btc_output::Model as BitcoinOutputModel, contact::ContactModel, invoice::Model as InvoiceModel,
    ln_address::Model as LnAddressModel, payment::Model as PaymentModel, wallet::Model as WalletModel,
};

const ASSERTION_MSG: &str = "should parse successfully by assertion";

impl From<AssetModel> for Asset {
    fn from(model: AssetModel) -> Self {
        Asset {
            id: model.id,
            code: model.code,
            name: model.name,
            protocol: model.protocol.parse().expect(ASSERTION_MSG),
            network: model.network.parse().expect(ASSERTION_MSG),
            asset_ref: model.asset_ref,
            display_ticker: model.display_ticker,
            decimals: model.decimals,
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
        }
    }
}

/// Maps the API ordering direction to sea-orm's `Order`. A free function rather
/// than a `From` impl because both types are foreign here (orphan rule).
pub fn sea_order(direction: &OrderDirection) -> Order {
    match direction {
        OrderDirection::Asc => Order::Asc,
        OrderDirection::Desc => Order::Desc,
    }
}

impl From<AccountModel> for Account {
    fn from(model: AccountModel) -> Self {
        Account {
            id: model.id,
            display_name: model.display_name,
            identity: None,
            permissions: serde_json::from_value(model.permissions).expect(ASSERTION_MSG),
            preferences: None,
            wallets: Vec::new(),
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
        }
    }
}

impl From<AuthIdentityModel> for AuthIdentity {
    fn from(model: AuthIdentityModel) -> Self {
        AuthIdentity {
            id: model.id,
            provider: model.provider.parse().expect(ASSERTION_MSG),
            subject: model.subject,
            created_at: model.created_at.and_utc(),
        }
    }
}

impl From<AccountPreferenceModel> for AccountPreferences {
    fn from(model: AccountPreferenceModel) -> Self {
        AccountPreferences {
            dashboard_settings: model.dashboard_settings,
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
        }
    }
}

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
            status,
            fee_msat: model.fee_msat.map(|v| v as u64),
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
        let ledger = model.ledger.parse().expect(ASSERTION_MSG);

        let lightning = (ledger == Ledger::Lightning).then(|| LnPayment {
            ln_address: model.ln_address.clone(),
            payment_hash: model
                .payment_hash
                .clone()
                .expect("payment_hash should exist for Lightning payment"),
            payment_preimage: model.payment_preimage.clone(),
            metadata: model.metadata.clone(),
            success_action: serde_json::from_value(model.success_action.clone().unwrap_or_default()).ok(),
            raw_success_action: serde_json::from_value(model.raw_success_action.clone().unwrap_or_default()).ok(),
        });

        let bitcoin = (ledger == Ledger::Onchain).then(|| BtcPayment {
            address: model
                .btc_address
                .clone()
                .expect("destination address should exist for On-chain payment"),
            txid: model
                .payment_hash
                .clone()
                .expect("payment_hash (txid) should exist for On-chain payment"),
            block_height: model.btc_block_height.map(|h| h as u32),
        });

        let internal = (ledger == Ledger::Internal).then(|| InternalPayment {
            ln_address: model.ln_address.clone(),
            btc_address: model.btc_address.clone(),
            payment_hash: model.payment_hash.clone(),
        });

        Payment {
            id: model.id,
            wallet_id: model.wallet_id,
            error: model.error,
            amount_msat: model.amount_msat as u64,
            fee_msat: model.fee_msat.map(|v| v as u64),
            reserved_amount: model.reserved_amount as u64,
            payment_time: model.payment_time.map(|t| t.and_utc()),
            status: model.status.parse().expect(ASSERTION_MSG),
            ledger,
            description: model.description,
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
            lightning,
            bitcoin,
            internal,
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
            account_id: model.account_id,
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
            account_id: model.account_id,
            asset_id: model.asset_id,
            label: model.label,
            balance: Balance {
                available_msat: model.available_amount,
                reserved_msat: model.reserved_amount as u64,
                ..Default::default()
            },
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
            account_id: model.account_id,
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

impl From<BitcoinOutputModel> for BtcOutput {
    fn from(model: BitcoinOutputModel) -> Self {
        BtcOutput {
            id: model.id,
            outpoint: model.outpoint,
            txid: model.txid,
            output_index: model.output_index as u32,
            address: model.address,
            amount_sat: model.amount_sat as u64,
            status: model.status.parse().expect(ASSERTION_MSG),
            block_height: model.block_height.map(|h| h as u32),
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.map(|t| t.and_utc()),
        }
    }
}

impl From<BitcoinAddressModel> for BtcAddress {
    fn from(model: BitcoinAddressModel) -> Self {
        BtcAddress {
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

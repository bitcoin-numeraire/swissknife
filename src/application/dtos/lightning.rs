use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::domains::lightning::entities::LightningAddress;
use crate::domains::lightning::entities::LightningInvoice;

#[derive(Debug, Deserialize)]
pub struct SendPaymentRequest {
    pub input: String,
    pub amount_msat: Option<u64>,
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterLightningAddressRequest {
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct ProcessEventRequest {
    pub template: String,
    pub data: EventDataRequest,
}

#[derive(Debug, Deserialize)]
pub struct EventDataRequest {
    pub payment_hash: String,
}

#[derive(Debug, Serialize)]
pub struct LightningAddressResponse {
    pub id: Uuid,
    pub user_id: String,
    pub username: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

impl From<LightningAddress> for LightningAddressResponse {
    fn from(address: LightningAddress) -> Self {
        Self {
            id: address.id,
            user_id: address.user_id,
            username: address.username,
            active: address.active,
            created_at: address.created_at,
            updated_at: address.updated_at,
            deleted_at: address.deleted_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LightningInvoiceResponse {
    pub id: Uuid,
    pub lightning_address: Option<String>,
    pub bolt11: String,
    pub network: String,
    pub payee_pubkey: String,
    pub payment_hash: String,
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub amount_msat: Option<i64>,
    pub timestamp: i64,
    pub expiry: i64,
    pub min_final_cltv_expiry_delta: i64,
    pub fee_msat: Option<i64>,
    pub payment_time: Option<i64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<LightningInvoice> for LightningInvoiceResponse {
    fn from(invoice: LightningInvoice) -> Self {
        Self {
            id: invoice.id.unwrap(), // Can't be empty on the API layer
            lightning_address: invoice.lightning_address,
            bolt11: invoice.bolt11,
            network: invoice.network,
            payee_pubkey: invoice.payee_pubkey,
            payment_hash: invoice.payment_hash,
            description: invoice.description,
            description_hash: invoice.description_hash,
            amount_msat: invoice.amount_msat,
            timestamp: invoice.timestamp,
            expiry: invoice.expiry,
            min_final_cltv_expiry_delta: invoice.min_final_cltv_expiry_delta,
            fee_msat: invoice.fee_msat,
            payment_time: invoice.payment_time,
            status: invoice.status,
            created_at: invoice.created_at.unwrap(), // Can't be empty on the API layer
            updated_at: invoice.updated_at,
        }
    }
}

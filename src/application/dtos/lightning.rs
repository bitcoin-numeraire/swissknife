use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::domains::lightning::entities::{
    LightningAddress, LightningInvoice, LightningInvoiceStatus, LightningPayment,
    LightningPaymentStatus,
};

#[derive(Debug, Deserialize)]
pub struct NewInvoiceRequest {
    pub amount_msat: u64,
    pub description: Option<String>,
    pub expiry: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SendPaymentRequest {
    pub input: String,
    pub amount_msat: Option<u64>,
    pub comment: Option<String>,
}

// Part of the lightning types because this is the payload to send from the node with a swap service
#[derive(Debug, Deserialize)]
pub struct SendOnchainPaymentRequest {
    pub amount_msat: u64,
    pub recipient_address: String,
    pub feerate: u32,
}

#[derive(Debug, Deserialize)]
pub struct RedeemOnchainRequest {
    pub to_address: String,
    pub feerate: u32,
}

#[derive(Debug, Deserialize)]
pub struct RegisterLightningAddressRequest {
    pub user_id: String,
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
    pub payment_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lightning_address: Option<String>,
    pub bolt11: String,
    pub network: String,
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub amount_msat: Option<u64>,
    pub timestamp: DateTime<Utc>,
    pub expiry: u64,
    pub payee_pubkey: String,
    pub min_final_cltv_expiry_delta: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_msat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_time: Option<DateTime<Utc>>,
    pub status: LightningInvoiceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

impl From<LightningInvoice> for LightningInvoiceResponse {
    fn from(invoice: LightningInvoice) -> Self {
        Self {
            id: invoice.id,
            payment_hash: invoice.payment_hash,
            lightning_address: invoice.lightning_address,
            bolt11: invoice.bolt11,
            network: invoice.network,
            description: invoice.description,
            description_hash: invoice.description_hash,
            amount_msat: invoice.amount_msat,
            timestamp: invoice.timestamp,
            expiry: invoice.expiry.as_secs(),
            payee_pubkey: invoice.payee_pubkey,
            min_final_cltv_expiry_delta: invoice.min_final_cltv_expiry_delta,
            fee_msat: invoice.fee_msat,
            payment_time: invoice.payment_time,
            status: invoice.status,
            label: invoice.label,
            created_at: invoice.created_at,
            updated_at: invoice.updated_at,
            expires_at: invoice.expires_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LightningPaymentResponse {
    pub id: Uuid,
    pub lightning_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub amount_msat: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_msat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_time: Option<DateTime<Utc>>,
    pub status: LightningPaymentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_action: Option<Value>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<LightningPayment> for LightningPaymentResponse {
    fn from(payment: LightningPayment) -> Self {
        Self {
            id: payment.id,
            lightning_address: payment.lightning_address,
            payment_hash: payment.payment_hash,
            error: payment.error,
            amount_msat: payment.amount_msat,
            fee_msat: payment.fee_msat,
            payment_time: payment.payment_time,
            status: payment.status,
            description: payment.description,
            metadata: payment.metadata,
            success_action: payment.success_action,
            created_at: payment.created_at,
            updated_at: payment.updated_at,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LightningInvoiceFilter {
    pub user_id: Option<String>,
    pub status: Option<LightningInvoiceStatus>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LightningAddressFilter {
    pub username: Option<String>,
    pub user_id: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub id: Option<Uuid>,
}

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::domains::lightning::entities::LightningAddress;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LightningWellKnownResponse {
    pub callback: String, // The URL from LN SERVICE which will accept the pay request parameters
    pub max_sendable: u64, // Max amount in milli-satoshis LN SERVICE is willing to receive
    pub min_sendable: u64, // Min amount in milli-satoshis LN SERVICE is willing to receive, can not be less than 1 or more than `maxSendable`
    pub metadata: String, // Metadata json which must be presented as raw string here, this is required to pass signature verification at a later step
    pub comment_allowed: Option<u8>, // Optional number of characters accepted for the `comment` query parameter on subsequent callback, defaults to 0 if not provided. (no comment allowed)
    pub withdraw_link: Option<String>, // Optional lnurl-withdraw link
    pub tag: String,                 // Type of LNURL
}

#[derive(Deserialize)]
pub struct LightningInvoiceQueryParams {
    pub amount: u64, // Amount in milli-satoshis
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LightningInvoiceResponse {
    pub pr: String,                            // bech32-serialized lightning invoice
    pub success_action: Option<SuccessAction>, // An optional action to be executed after successfully paying an invoice
    pub disposable: Option<bool>, // An optional flag to let a wallet know whether to persist the link from step 1, if null should be interpreted as true
    pub routes: Vec<String>, // array with payment routes, should be left empty if no routes are to be provided
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessAction {
    pub tag: String,             // action type (url, message, aes, ...)
    pub message: Option<String>, // rest of fields depends on tag value
}

#[derive(Debug, Deserialize)]
pub struct RegisterLightningAddressRequest {
    pub username: String,
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

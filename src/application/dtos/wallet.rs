use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domains::{
    ln_address::LnAddress,
    wallet::{Balance, Contact, Wallet},
};

use super::{InvoiceResponse, PaymentResponse};

/// Register Wallet Request
#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct RegisterWalletRequest {
    /// User ID. Should ideally be registered in your Auth provider.
    pub user_id: String,
}

#[derive(Serialize, ToSchema)]
pub struct WalletResponse {
    /// Internal ID
    pub id: Uuid,
    /// User ID. Populated from the Authentication method,  such as JWT subject
    pub user_id: String,
    /// Lightning Address
    pub ln_address: Option<LnAddress>,
    /// User Balance
    pub balance: Balance,
    /// List of payments
    pub payments: Vec<PaymentResponse>,
    /// List of Invoices
    pub invoices: Vec<InvoiceResponse>,
    /// List of contacts
    pub contacts: Vec<Contact>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Date of update in database
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Wallet> for WalletResponse {
    fn from(wallet: Wallet) -> Self {
        WalletResponse {
            id: wallet.id,
            user_id: wallet.user_id,
            ln_address: wallet.ln_address,
            balance: wallet.balance,
            payments: wallet.payments.into_iter().map(Into::into).collect(),
            invoices: wallet.invoices.into_iter().map(Into::into).collect(),
            contacts: wallet.contacts,
            created_at: wallet.created_at,
            updated_at: wallet.updated_at,
        }
    }
}

use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{Invoice, LnAddress, Payment};

#[derive(Debug, Clone, Serialize, Default, ToSchema)]
pub struct Balance {
    /// Total amount received
    #[schema(example = 1000000000)]
    pub received_msat: u64,

    /// Total amount sent (settled outgoing payments)
    #[schema(example = 10000000)]
    pub sent_msat: u64,

    /// Total fees paid
    #[schema(example = 1000)]
    pub fees_paid_msat: u64,

    /// Amount reserved for pending outgoing payments
    #[schema(example = 2000)]
    pub reserved_msat: u64,

    /// Amount available to spend.
    #[schema(example = 989999000)]
    pub available_msat: i64,
}

#[derive(Debug, Clone, Serialize, Default, ToSchema)]
pub struct Contact {
    /// Lightning Address
    #[schema(example = "dario_nakamoto@numeraire.tech")]
    pub ln_address: String,

    /// Date of first payment to this contact
    pub contact_since: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct Wallet {
    /// Internal ID
    pub id: Uuid,
    /// User ID. Populated from the Authentication method,  such as JWT subject
    pub user_id: String,
    /// Lightning Address
    pub ln_address: Option<LnAddress>,
    /// User Balance
    pub balance: Balance,
    /// List of payments
    pub payments: Vec<Payment>,
    /// List of Invoices
    pub invoices: Vec<Invoice>,
    /// List of contacts
    pub contacts: Vec<Contact>,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Date of update in database
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct WalletOverview {
    /// Internal ID
    pub id: Uuid,
    /// User ID. Populated from the Authentication method,  such as JWT subject
    pub user_id: String,
    /// Lightning Address
    pub ln_address: Option<LnAddress>,
    /// User Balance
    pub balance: Balance,
    /// Number of payments
    pub n_payments: u32,
    /// Number of invoices
    pub n_invoices: u32,
    /// Number of contacts
    pub n_contacts: u32,
    /// Date of creation in database
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Date of update in database
    pub updated_at: Option<DateTime<Utc>>,
}

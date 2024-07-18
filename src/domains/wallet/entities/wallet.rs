use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    application::entities::OrderDirection,
    domains::{invoice::Invoice, ln_address::LnAddress, payment::Payment},
};

use super::{Balance, Contact};

#[derive(Debug, Clone, Default)]
pub struct Wallet {
    pub id: Uuid,
    pub user_id: String,
    pub ln_address: Option<LnAddress>,
    pub balance: Balance,
    pub payments: Vec<Payment>,
    pub invoices: Vec<Invoice>,
    pub contacts: Vec<Contact>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct WalletOverview {
    /// Internal ID
    pub id: Uuid,
    /// User ID. Populated from the Authentication method,  such as JWT subject
    pub user_id: String,
    /// Lightning Address ID
    pub ln_address_id: Option<Uuid>,
    /// Lightning Address Username
    pub ln_address_username: Option<String>,
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

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default, IntoParams)]
pub struct WalletFilter {
    /// Total amount of results to return
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<u64>,
    /// Offset where to start returning results
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<u64>,
    /// List of IDs
    pub ids: Option<Vec<Uuid>>,
    /// User ID. Automatically populated with your ID
    pub user_id: Option<String>,
    /// Direction of the ordering of results
    #[serde(default)]
    pub order_direction: OrderDirection,
}

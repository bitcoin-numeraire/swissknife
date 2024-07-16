use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    application::entities::Currency,
    domains::wallet::{Contact, UserBalance, Wallet},
};

use super::{InvoiceResponse, PaymentResponse};

#[derive(Serialize, ToSchema)]
pub struct WalletResponse {
    /// Internal ID
    pub id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Currency
    pub currency: Currency,
    /// User Balance
    pub user_balance: UserBalance,
    /// List of payments
    pub payments: Vec<PaymentResponse>,
    /// Lit of Invoices
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
            currency: wallet.currency,
            user_balance: wallet.user_balance,
            payments: wallet.payments.into_iter().map(Into::into).collect(),
            invoices: wallet.invoices.into_iter().map(Into::into).collect(),
            contacts: wallet.contacts,
            created_at: wallet.created_at,
            updated_at: wallet.updated_at,
        }
    }
}

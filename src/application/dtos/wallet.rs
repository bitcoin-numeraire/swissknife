use serde::Serialize;
use utoipa::ToSchema;

use crate::domains::{
    lightning::entities::LnAddress,
    wallet::entities::{Contact, UserBalance, Wallet},
};

use super::{InvoiceResponse, PaymentResponse};

#[derive(Serialize, ToSchema)]
pub struct WalletResponse {
    /// User Balance
    pub user_balance: UserBalance,
    /// List of payments
    pub payments: Vec<PaymentResponse>,
    /// Lit of Invoices
    pub invoices: Vec<InvoiceResponse>,
    /// Lightning Address
    pub ln_address: Option<LnAddress>,
    /// List of contacts
    pub contacts: Vec<Contact>,
}

impl From<Wallet> for WalletResponse {
    fn from(wallet: Wallet) -> Self {
        WalletResponse {
            user_balance: wallet.user_balance,
            payments: wallet.payments.into_iter().map(Into::into).collect(),
            invoices: wallet.invoices.into_iter().map(Into::into).collect(),
            ln_address: wallet.ln_address,
            contacts: wallet.contacts,
        }
    }
}

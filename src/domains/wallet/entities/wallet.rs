use serde::Serialize;
use utoipa::ToSchema;

use crate::domains::{
    invoices::entities::Invoice, lightning::entities::LnAddress, payments::entities::Payment,
};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Wallet {
    /// User Balance
    pub user_balance: UserBalance,
    /// List of payments
    pub payments: Vec<Payment>,
    /// Lit of Invoices
    pub invoices: Vec<Invoice>,
    /// Lightning Address
    pub ln_address: Option<LnAddress>,
}

#[derive(Debug, Clone, Serialize, Default, ToSchema)]
pub struct UserBalance {
    /// Total amount received
    #[schema(example = 1000000000)]
    pub received_msat: u64,

    /// Total amount sent
    #[schema(example = 10000000)]
    pub sent_msat: u64,

    /// Total fees paid
    pub fees_paid_msat: u64,
    #[schema(example = 1000)]

    /// Amount available to spend
    #[schema(example = 989999000)]
    pub available_msat: i64,
}

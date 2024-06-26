use serde::Serialize;

use crate::domains::{
    invoices::entities::Invoice, lightning::entities::LnAddress, payments::entities::Payment,
};

#[derive(Debug, Clone, Serialize)]
pub struct Wallet {
    pub user_balance: UserBalance,
    pub payments: Vec<Payment>,
    pub invoices: Vec<Invoice>,
    pub ln_address: Option<LnAddress>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct UserBalance {
    pub received_msat: u64,
    pub sent_msat: u64,
    pub fees_paid_msat: u64,
    pub available_msat: i64,
}

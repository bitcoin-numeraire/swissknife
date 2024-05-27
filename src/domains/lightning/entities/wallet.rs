use serde::Serialize;

use super::{LightningAddress, LightningInvoice, LightningPayment};

#[derive(Debug, Clone, Serialize)]
pub struct Wallet {
    pub user_balance: UserBalance,
    pub payments: Vec<LightningPayment>,
    pub invoices: Vec<LightningInvoice>,
    pub address: Option<LightningAddress>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct UserBalance {
    pub received_msat: u64,
    pub sent_msat: u64,
    pub fees_paid_msat: u64,
    pub available_msat: i64,
}

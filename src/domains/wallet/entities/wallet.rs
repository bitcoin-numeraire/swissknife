use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    application::entities::Currency,
    domains::{invoice::Invoice, payment::Payment},
};

use super::Contact;

#[derive(Debug, Clone, Default)]
pub struct Wallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub currency: Currency,
    pub balance: Balance,
    pub payments: Vec<Payment>,
    pub invoices: Vec<Invoice>,
    pub contacts: Vec<Contact>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Default, ToSchema)]
pub struct Balance {
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

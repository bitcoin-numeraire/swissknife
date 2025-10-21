use sea_orm::{prelude::DateTime, FromQueryResult};
use uuid::Uuid;

#[derive(Debug, Clone, FromQueryResult)]
pub struct WalletOverviewModel {
    pub id: Uuid,
    pub user_id: String,
    pub received_msat: Option<i64>,
    pub sent_msat: Option<i64>,
    pub fees_paid_msat: Option<i64>,
    pub n_payments: i64,
    pub n_invoices: i64,
    pub n_contacts: i64,
    pub created_at: DateTime,
    pub updated_at: Option<DateTime>,
}

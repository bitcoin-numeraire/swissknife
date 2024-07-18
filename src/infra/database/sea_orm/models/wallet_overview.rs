use chrono::{DateTime, Utc};
use sea_orm::FromQueryResult;
use uuid::Uuid;

#[derive(Debug, FromQueryResult)]
pub struct WalletOverviewModel {
    pub id: Uuid,
    pub user_id: String,
    pub ln_address_id: Option<Uuid>,
    pub ln_address_username: Option<String>,
    pub received_msat: i64,
    pub sent_msat: i64,
    pub fees_paid_msat: i64,
    pub n_payments: i64,
    pub n_invoices: i64,
    pub n_contacts: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

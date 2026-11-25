use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct WalletStatement {
    pub wallet_id: Uuid,
    pub asset_id: Uuid,
    pub network: String,
    pub entries: Vec<StatementEntry>,
}

pub struct StatementEntry {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub payment_time: Option<DateTime<Utc>>,
    pub direction: String,
    pub ledger: String,
    pub status: String,
    pub amount_msat: Option<i64>,
    pub fee_msat: Option<i64>,
    pub description: Option<String>,
    pub credit_purpose: Option<String>,
}

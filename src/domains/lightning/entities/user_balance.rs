use serde::Serialize;

#[derive(Serialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct UserBalance {
    pub received_msat: u64,
    pub sent_msat: u64,
    pub fees_paid_msat: u64,
    pub available_msat: i64,
}

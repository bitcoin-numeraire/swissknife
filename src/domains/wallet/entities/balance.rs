use serde::Serialize;
use utoipa::ToSchema;

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

impl Balance {
    pub fn new(received_msat: i64, sent_msat: i64, fees_paid_msat: i64) -> Self {
        Self {
            received_msat: received_msat as u64,
            sent_msat: sent_msat as u64,
            fees_paid_msat: fees_paid_msat as u64,
            available_msat: received_msat - (sent_msat + fees_paid_msat),
        }
    }
}

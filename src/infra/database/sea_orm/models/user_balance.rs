use sea_orm::FromQueryResult;

#[derive(Debug, FromQueryResult)]
pub struct UserBalanceModel {
    pub received_msat: i64,
    pub sent_msat: i64,
    pub fees_paid_msat: i64,
}

use sea_orm::FromQueryResult;

use crate::domains::wallet::entities::UserBalance;

#[derive(Debug, FromQueryResult)]
pub(crate) struct UserBalanceModel {
    pub received_msat: i64,
    pub sent_msat: i64,
    pub fees_paid_msat: i64,
}

impl From<UserBalanceModel> for UserBalance {
    fn from(model: UserBalanceModel) -> Self {
        UserBalance {
            received_msat: model.received_msat as u64,
            sent_msat: model.sent_msat as u64,
            fees_paid_msat: model.fees_paid_msat as u64,
            available_msat: model.received_msat - (model.sent_msat + model.fees_paid_msat),
        }
    }
}

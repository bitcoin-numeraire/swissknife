use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountIdentity {
    pub account_id: Uuid,
    pub provider: String,
    pub subject: String,
}

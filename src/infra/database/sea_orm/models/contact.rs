use sea_orm::{prelude::DateTime, FromQueryResult};

#[derive(Debug, FromQueryResult)]
pub struct ContactModel {
    pub ln_address: String,
    pub contact_since: Option<DateTime>,
}

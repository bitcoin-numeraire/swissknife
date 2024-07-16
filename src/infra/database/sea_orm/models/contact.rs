use sea_orm::{prelude::DateTimeUtc, FromQueryResult};

#[derive(Debug, FromQueryResult)]
pub struct ContactModel {
    pub ln_address: String,
    pub contact_since: DateTimeUtc,
}

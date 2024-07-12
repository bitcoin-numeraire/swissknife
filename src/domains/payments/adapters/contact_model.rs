use sea_orm::{prelude::DateTimeUtc, FromQueryResult};

use crate::domains::wallet::entities::Contact;

#[derive(Debug, FromQueryResult)]
pub(crate) struct ContactModel {
    pub ln_address: String,
    pub contact_since: DateTimeUtc,
}

impl From<ContactModel> for Contact {
    fn from(model: ContactModel) -> Self {
        Contact {
            ln_address: model.ln_address,
            contact_since: model.contact_since,
        }
    }
}

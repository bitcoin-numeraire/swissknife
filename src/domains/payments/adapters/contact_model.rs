use sea_orm::FromQueryResult;

use crate::domains::wallet::entities::Contact;

#[derive(Debug, FromQueryResult)]
pub(crate) struct ContactModel {
    pub ln_address: String,
}

impl From<ContactModel> for Contact {
    fn from(model: ContactModel) -> Self {
        Contact {
            ln_address: model.ln_address,
        }
    }
}

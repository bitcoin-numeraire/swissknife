mod agent_key_repository;
mod config;
mod models;
mod offer_repository;
mod payment_approval;
mod payment_command;
mod repositories;
mod spending_policy_repository;
mod store;
mod types;
mod uow;
mod withdrawal_command;

#[cfg(all(test, feature = "itest"))]
mod uow_tests;

pub use config::*;
pub use repositories::*;
pub use store::*;
pub use uow::*;

pub(crate) use types::{amount_to_db, sea_order};
fn wallet_deletion_error(error: sea_orm::DbErr) -> crate::application::errors::DatabaseError {
    use crate::application::errors::DatabaseError;
    let message = error.to_string();
    if message.contains("swissknife_offer_still_payable") {
        DatabaseError::PayableOffer
    } else {
        DatabaseError::Delete(message)
    }
}

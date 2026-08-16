use serde_json::Value;
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::{invoice::Invoice, payment::Payment},
};

pub use swissknife_types::{ClientEvent, ClientEventType};

#[derive(Clone, Debug)]
pub struct NewClientEvent {
    pub event_type: ClientEventType,
    pub wallet_id: Uuid,
    pub resource_id: Uuid,
    pub data: Value,
}

impl NewClientEvent {
    pub fn invoice_paid(invoice: &Invoice) -> Result<Self, DatabaseError> {
        Ok(Self {
            event_type: ClientEventType::InvoicePaid,
            wallet_id: invoice.wallet_id,
            resource_id: invoice.id,
            data: serde_json::to_value(invoice).map_err(|e| DatabaseError::Insert(e.to_string()))?,
        })
    }

    pub fn payment(payment: &Payment) -> Result<Self, DatabaseError> {
        let event_type = match payment.status {
            crate::domains::payment::PaymentStatus::Settled => ClientEventType::PaymentSettled,
            crate::domains::payment::PaymentStatus::Failed => ClientEventType::PaymentFailed,
            _ => {
                return Err(DatabaseError::Insert(format!(
                    "cannot emit a terminal event for payment {} in status {}",
                    payment.id, payment.status
                )))
            }
        };

        Ok(Self {
            event_type,
            wallet_id: payment.wallet_id,
            resource_id: payment.id,
            data: serde_json::to_value(payment).map_err(|e| DatabaseError::Insert(e.to_string()))?,
        })
    }
}

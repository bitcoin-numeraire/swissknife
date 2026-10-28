use super::Payment;
use crate::application::errors::{ApplicationError, DataError};

/// Provider replies and event/replay paths share one authorization rule. Old
/// reservation buffers cannot establish a historical maximum fee, so only a
/// recorded fee authorization imposes a ceiling. New sends always record it.
pub(crate) fn validate_lightning_settlement(
    payment: &Payment,
    amount_msat: u64,
    fee_msat: Option<u64>,
) -> Result<(), ApplicationError> {
    let fee_valid = fee_msat.is_some_and(|fee| {
        amount_msat.checked_add(fee).is_some() && payment.authorized_fee_msat.is_none_or(|limit| fee <= limit)
    });
    if amount_msat != payment.amount_msat || !fee_valid {
        return Err(DataError::Inconsistency(format!(
            "Provider settlement for payment {} disagrees with its principal or recorded fee authorization; ledger unchanged",
            payment.id
        )).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_authorization_is_independent_of_the_reservation_buffer() {
        let mut payment = Payment {
            amount_msat: 1_000,
            reserved_amount: 1_500,
            authorized_fee_msat: Some(100),
            ..Default::default()
        };
        assert!(validate_lightning_settlement(&payment, 1_000, Some(101)).is_err());
        payment.reserved_amount = 0;
        assert!(validate_lightning_settlement(&payment, 1_000, Some(100)).is_ok());
    }

    #[test]
    fn unknown_historical_fee_limits_are_not_invented_from_old_buffers() {
        let payment = Payment {
            amount_msat: 1_000,
            reserved_amount: 1_010,
            ..Default::default()
        };
        assert!(validate_lightning_settlement(&payment, 1_000, Some(50)).is_ok());
        assert!(validate_lightning_settlement(&payment, 999, Some(50)).is_err());
        assert!(validate_lightning_settlement(&payment, 1_000, None).is_err());
    }
}

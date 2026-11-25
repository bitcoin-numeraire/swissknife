use std::str::FromStr;

use bitcoin::Address;
use swissknife_types::OnchainFeePreview;

use crate::application::errors::BitcoinError;

/// Version, both compact counts, locktime, marker and flag. CLN adds the selected inputs.
const SEGWIT_TRANSACTION_WEIGHT: u32 = 42;
/// CLN txprepare returns change to P2TR. An output has 8 value bytes and 35 serialized script bytes.
const CHANGE_OUTPUT_WEIGHT: u64 = 4 * (8 + 1 + 34);
const P2TR_DUST_SAT: u64 = 330;

pub(super) fn start_weight(address: &str) -> Result<u32, BitcoinError> {
    let address = Address::from_str(address).map_err(|e| BitcoinError::Address(e.to_string()))?;
    // The application already validated the destination network.
    let script = address.assume_checked().script_pubkey();
    let output = bitcoin::TxOut {
        value: bitcoin::Amount::ZERO,
        script_pubkey: script,
    };
    Ok(SEGWIT_TRANSACTION_WEIGHT + output.weight().to_wu() as u32)
}

pub(super) fn preview(rate: u32, weight: u32, excess_msat: u64) -> Result<OnchainFeePreview, BitcoinError> {
    // fundpsbt leaves excess unallocated. Match CLN's txprepare change calculation
    // without advancing its change-address index. CLN rounds total fees down to sats.
    let weight = u64::from(weight);
    let rate = u64::from(rate);
    let base_fee = rate * weight / 1000;
    let with_change = rate
        .checked_mul(weight + CHANGE_OUTPUT_WEIGHT)
        .ok_or_else(|| BitcoinError::FeeEstimate("Invalid transaction weight or rate".into()))?
        / 1000;
    let excess_sat = excess_msat / 1000;
    let fee_sat = if excess_sat.saturating_sub(with_change - base_fee) >= P2TR_DUST_SAT {
        with_change
    } else {
        base_fee + excess_sat
    };
    Ok(OnchainFeePreview {
        fee_sat,
        fee_rate_sat_per_kw: rate,
        target_blocks: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_the_real_recipient_script_and_optional_taproot_change() {
        assert_eq!(start_weight("1BoatSLRHtKNngkdXEeobR76b53LETtpyT").unwrap(), 42 + 34 * 4);
        // 751 sat/kw demonstrates the difference between rounding a total and rounding each output.
        assert_eq!(preview(751, 455, 500_000).unwrap().fee_sat, 470);
        assert_eq!(preview(751, 455, 458_000).unwrap().fee_sat, 341 + 458);
        assert_eq!(preview(751, 455, 459_000).unwrap().fee_sat, 470);
        assert_eq!(preview(751, 455, 0).unwrap().fee_sat, 341);
    }
}

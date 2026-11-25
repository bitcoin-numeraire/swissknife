use std::str::FromStr;

use bitcoin::{Address, Amount, TxOut};

use crate::{
    application::errors::BitcoinError, domains::bitcoin::BtcPreparedTransaction,
    infra::lightning::bitcoin_utils::parse_psbt,
};

/// A reservation whose reply is lost expires without another worker releasing an unrelated input.
pub(super) const RESERVATION_BLOCKS: u32 = 6;

pub(super) fn destination(address: &str, amount_sat: u64) -> Result<(TxOut, u32, u64), BitcoinError> {
    let script = Address::from_str(address)
        .map_err(|e| BitcoinError::Address(e.to_string()))?
        .assume_checked()
        .script_pubkey();
    let expected = TxOut {
        value: Amount::from_sat(amount_sat),
        script_pubkey: script,
    };
    // Transaction overhead including marker/flag, plus this output. CLN accounts for selected inputs and change.
    let start_weight = 42 + expected.weight().to_wu() as u32;
    let msat = amount_sat
        .checked_mul(1000)
        .ok_or_else(|| BitcoinError::PrepareTransaction("Amount exceeds the supported range".into()))?;
    Ok((expected, start_weight, msat))
}

pub(super) fn plan(
    psbt_v0: &str,
    psbt: String,
    outnum: u32,
    expected: TxOut,
) -> Result<BtcPreparedTransaction, BitcoinError> {
    let decoded = parse_psbt(psbt_v0)?;
    if decoded.unsigned_tx.output.get(outnum as usize) != Some(&expected) {
        return Err(BitcoinError::PrepareTransaction(
            "Funded plan does not contain the requested output".into(),
        ));
    }
    // The txid of native SegWit inputs is independent of their witness signatures.
    for prevout in decoded.iter_funding_utxos() {
        if !prevout
            .map_err(|e| BitcoinError::ParsePsbt(e.to_string()))?
            .script_pubkey
            .is_witness_program()
        {
            return Err(BitcoinError::PrepareTransaction(
                "Core Lightning returned a wrapped input".into(),
            ));
        }
    }
    Ok(BtcPreparedTransaction {
        txid: decoded.unsigned_tx.compute_txid().to_string(),
        fee_sat: decoded
            .fee()
            .map_err(|e| BitcoinError::ParsePsbt(e.to_string()))?
            .to_sat(),
        psbt,
        locked_utxos: Vec::new(),
    })
}

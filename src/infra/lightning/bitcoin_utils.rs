use base64::{engine::general_purpose::STANDARD, Engine};
use bitcoin::{consensus::encode::deserialize, psbt::Psbt, Amount, Transaction};

use crate::application::errors::BitcoinError;

pub fn psbt_fee_sat(psbt_base64: &str) -> Result<u64, BitcoinError> {
    let psbt_bytes = STANDARD
        .decode(psbt_base64)
        .map_err(|e| BitcoinError::Transaction(format!("Failed to decode PSBT: {e}")))?;
    let psbt = Psbt::deserialize(&psbt_bytes)
        .map_err(|e| BitcoinError::Transaction(format!("Failed to deserialize PSBT: {e}")))?;

    let mut input_sum = Amount::from_sat(0);
    for (index, input) in psbt.inputs.iter().enumerate() {
        if let Some(utxo) = input.witness_utxo.as_ref() {
            input_sum += utxo.value;
            continue;
        }

        if let Some(prev_tx) = input.non_witness_utxo.as_ref() {
            let vout = psbt
                .unsigned_tx
                .input
                .get(index)
                .map(|input| input.previous_output.vout as usize)
                .ok_or_else(|| BitcoinError::Transaction("PSBT input index missing previous output".to_string()))?;

            let prev_output = prev_tx
                .output
                .get(vout)
                .ok_or_else(|| BitcoinError::Transaction("PSBT missing referenced UTXO output".to_string()))?;
            input_sum += prev_output.value;
            continue;
        }

        return Err(BitcoinError::Transaction("PSBT input missing UTXO data".to_string()));
    }

    let output_sum: Amount = psbt.unsigned_tx.output.iter().map(|out| out.value).sum();
    let fee = input_sum
        .checked_sub(output_sum)
        .ok_or_else(|| BitcoinError::Transaction("PSBT fee calculation underflow".to_string()))?;

    Ok(fee.to_sat())
}

pub fn txid_from_raw_tx(raw_tx: &[u8]) -> Result<String, BitcoinError> {
    let tx: Transaction =
        deserialize(raw_tx).map_err(|e| BitcoinError::Transaction(format!("Failed to decode transaction: {e}")))?;
    Ok(tx.compute_txid().to_string())
}

use base64::{engine::general_purpose::STANDARD, Engine};
use bitcoin::psbt::Psbt;

use crate::application::errors::BitcoinError;

pub fn parse_psbt(psbt_base64: &str) -> Result<Psbt, BitcoinError> {
    let psbt_bytes = STANDARD
        .decode(psbt_base64)
        .map_err(|e| BitcoinError::ParsePsbt(e.to_string()))?;

    Psbt::deserialize(&psbt_bytes).map_err(|e| BitcoinError::ParsePsbt(e.to_string()))
}

/// Check that signing changed only input unlocking data. The final scriptSig can change the txid.
pub fn signed_transaction(
    prepared: &str,
    bytes: &[u8],
) -> Result<crate::domains::bitcoin::BtcBroadcastTransaction, BitcoinError> {
    let funded = parse_psbt(prepared)?.unsigned_tx;
    let signed: bitcoin::Transaction =
        bitcoin::consensus::deserialize(bytes).map_err(|e| BitcoinError::FinalizeTransaction(e.to_string()))?;
    if signed.version != funded.version
        || signed.lock_time != funded.lock_time
        || signed.output != funded.output
        || signed.input.len() != funded.input.len()
        || signed
            .input
            .iter()
            .zip(&funded.input)
            .any(|(a, b)| a.previous_output != b.previous_output || a.sequence != b.sequence)
    {
        return Err(BitcoinError::FinalizeTransaction(
            "Signed transaction differs from its funded template".into(),
        ));
    }
    Ok(crate::domains::bitcoin::BtcBroadcastTransaction::Signed(signed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{
        absolute::LockTime, consensus::serialize, transaction::Version, Amount, OutPoint, ScriptBuf, Sequence,
        Transaction, TxIn, TxOut, Witness,
    };

    #[test]
    fn finalized_input_scripts_determine_identity_and_preserve_the_funded_outputs() {
        let tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint::null(),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(12_000),
                script_pubkey: ScriptBuf::new(),
            }],
        };
        let psbt = STANDARD.encode(Psbt::from_unsigned_tx(tx.clone()).unwrap().serialize());
        let mut finalized = tx.clone();
        finalized.input[0].script_sig = ScriptBuf::from_bytes([vec![22, 0, 20], vec![7; 20]].concat());
        assert_ne!(tx.compute_txid(), finalized.compute_txid());
        let verified = signed_transaction(&psbt, &serialize(&finalized)).unwrap();
        assert_eq!(verified.txid(), finalized.compute_txid().to_string());
        finalized.output[0].value = Amount::from_sat(11_999);
        assert!(signed_transaction(&psbt, &serialize(&finalized)).is_err());
        assert!(signed_transaction(&psbt, &[]).is_err());
    }
}

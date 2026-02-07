use base64::{engine::general_purpose::STANDARD, Engine};
use bitcoin::psbt::Psbt;

use crate::application::errors::BitcoinError;

pub fn parse_psbt(psbt_base64: &str) -> Result<Psbt, BitcoinError> {
    let psbt_bytes = STANDARD
        .decode(psbt_base64)
        .map_err(|e| BitcoinError::ParsePsbt(e.to_string()))?;

    Psbt::deserialize(&psbt_bytes).map_err(|e| BitcoinError::ParsePsbt(e.to_string()))
}

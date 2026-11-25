mod lnd_grpc_client;
mod lnd_grpc_listener;
mod lnd_rest_client;
pub mod lnd_types;
mod lnd_websocket_listener;

pub use lnd_grpc_client::*;
pub use lnd_grpc_listener::LndGrpcListener;
pub use lnd_rest_client::*;
pub use lnd_websocket_listener::LndWebsocketListener;

fn verify_withdrawal_output(
    plan: &crate::domains::bitcoin::BtcPreparedTransaction,
    address: &str,
    sats: u64,
) -> Result<(), crate::application::errors::BitcoinError> {
    use crate::application::errors::BitcoinError;
    use std::str::FromStr;
    let expected = bitcoin::Address::from_str(address)
        .map_err(|e| BitcoinError::Address(e.to_string()))?
        .assume_checked()
        .script_pubkey();
    let tx = super::bitcoin_utils::parse_psbt(&plan.psbt)?.unsigned_tx;
    if tx
        .output
        .iter()
        .filter(|o| o.script_pubkey == expected && o.value.to_sat() == sats)
        .count()
        != 1
    {
        return Err(BitcoinError::PrepareTransaction(
            "Funded plan differs from the requested Bitcoin output".into(),
        ));
    }
    Ok(())
}

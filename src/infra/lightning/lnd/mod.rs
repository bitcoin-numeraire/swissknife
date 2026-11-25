mod lnd_grpc_client;
mod lnd_grpc_listener;
mod lnd_rest_client;
pub mod lnd_types;
mod lnd_websocket_listener;

pub use lnd_grpc_client::*;
pub use lnd_grpc_listener::LndGrpcListener;
pub use lnd_rest_client::*;
pub use lnd_websocket_listener::LndWebsocketListener;

fn onchain_fee_preview(
    fee_sat: i64,
    sat_per_vbyte: u64,
) -> Result<swissknife_types::OnchainFeePreview, crate::application::errors::BitcoinError> {
    use crate::application::errors::BitcoinError;
    Ok(swissknife_types::OnchainFeePreview {
        fee_sat: fee_sat
            .try_into()
            .map_err(|_| BitcoinError::FeeEstimate("Negative miner fee".into()))?,
        fee_rate_sat_per_kw: sat_per_vbyte
            .checked_mul(250)
            .ok_or_else(|| BitcoinError::FeeEstimate("Fee rate exceeds the storage range".into()))?,
        target_blocks: Some(1),
    })
}

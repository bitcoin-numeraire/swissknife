use crate::domains::bitcoin::BtcNetwork;

pub use swissknife_types::Asset;

pub const BITCOIN_PROTOCOL: &str = "bitcoin";
pub const NATIVE_ASSET_REF: &str = "native";

pub fn bitcoin_network_key(network: BtcNetwork) -> &'static str {
    match network {
        BtcNetwork::Bitcoin => "bitcoin/mainnet",
        BtcNetwork::Testnet => "bitcoin/testnet",
        BtcNetwork::Testnet4 => "bitcoin/testnet4",
        BtcNetwork::Regtest => "bitcoin/regtest",
        BtcNetwork::Simnet => "bitcoin/simnet",
        BtcNetwork::Signet => "bitcoin/signet",
    }
}

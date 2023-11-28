use std::sync::Arc;

use rgb_lib::{
    restore_keys,
    wallet::{DatabaseType, WalletData},
    Wallet,
};
use tokio::{sync::Mutex, task};

use crate::application::errors::{ApplicationError, AsyncError, ConfigError};

#[derive(Clone)]
pub struct BreezClientConfig {
    pub data_dir: String,
    pub mnemonic: String,
    pub electrum_url: String,
}

pub struct BreezClient {
    url: String,
    wallet: Arc<Mutex<Wallet>>,
}

impl BreezClient {
    pub async fn new(config: BreezClientConfig) -> Result<Self, ApplicationError> {
        let keys = restore_keys(rgb_lib::BitcoinNetwork::Regtest, config.mnemonic)
            .map_err(|e| ConfigError::Wallet(e.to_string()))?;

        let wallet_data = WalletData {
            bitcoin_network: rgb_lib::BitcoinNetwork::Regtest,
            database_type: DatabaseType::Sqlite,
            data_dir: config.data_dir,
            pubkey: keys.xpub,
            mnemonic: Some(keys.mnemonic),
            vanilla_keychain: None,
            max_allocations_per_utxo: 5,
        };

        // Offload the blocking Wallet::new call to a separate thread
        let wallet = task::spawn_blocking(move || Wallet::new(wallet_data))
            .await
            .map_err(AsyncError::from)?
            .map_err(|e| ConfigError::Wallet(e.to_string()))?;

        Ok(Self {
            url: config.electrum_url,
            wallet: Arc::new(Mutex::new(wallet)),
        })
    }
}

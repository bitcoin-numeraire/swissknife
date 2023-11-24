use std::sync::Arc;

use async_trait::async_trait;
use rgb_lib::{
    restore_keys,
    wallet::{DatabaseType, WalletData},
    Wallet,
};
use tokio::{sync::Mutex, task};

use crate::{
    adapters::rgb::RGBClient,
    application::errors::{ApplicationError, AsyncError, ConfigError, RGBError},
    domains::rgb::entities::RGBContract,
};

#[derive(Clone)]
pub struct RGBLibClientConfig {
    pub data_dir: String,
    pub mnemonic: String,
}

pub struct RGBLibClient {
    wallet: Arc<Mutex<Wallet>>,
}

impl RGBLibClient {
    pub async fn new(config: RGBLibClientConfig) -> Result<Self, ApplicationError> {
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
            wallet: Arc::new(Mutex::new(wallet)),
        })
    }
}

#[async_trait]
impl RGBClient for RGBLibClient {
    async fn issue_contract(
        &self,
        url: String,
        contract: RGBContract,
    ) -> Result<String, ApplicationError> {
        let mut wallet = self.wallet.lock().await;

        let test = wallet.get_address().unwrap();
        println!("wallet got: {}", test);

        let online = wallet
            .go_online(false, url.clone())
            .map_err(|e| RGBError::ContractIssuanceError(e.to_string()))?;

        println!("Online: {:?}", online);

        let contract = wallet
            .issue_asset_nia(
                online,
                contract.ticker,
                contract.name,
                contract.precision,
                contract.amounts,
            )
            .map_err(|e| RGBError::ContractIssuanceError(e.to_string()))?;

        println!("Contract issued: {:?}", contract);

        Ok(contract.asset_id)
    }
}

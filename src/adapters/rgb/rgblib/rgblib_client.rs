use std::sync::Arc;

use async_trait::async_trait;
use rgb_lib::{
    restore_keys,
    wallet::{DatabaseType, Online, Unspent, WalletData},
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
    pub electrum_url: String,
}

pub struct RGBLibClient {
    url: String,
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
            url: config.electrum_url,
            wallet: Arc::new(Mutex::new(wallet)),
        })
    }

    async fn online(&self) -> Result<Online, RGBError> {
        let mut wallet = self.wallet.lock().await;

        let online = wallet
            .go_online(false, self.url.clone())
            .map_err(|e| RGBError::Online(e.to_string()))?;

        Ok(online)
    }
}

#[async_trait]
impl RGBClient for RGBLibClient {
    async fn get_address(&self) -> Result<String, ApplicationError> {
        let wallet = self.wallet.lock().await;

        let address = wallet
            .get_address()
            .map_err(|e| RGBError::Address(e.to_string()))?;

        Ok(address)
    }

    async fn get_btc_balance(&self) -> Result<u64, ApplicationError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let balance = wallet
            .get_btc_balance(online)
            .map_err(|e| RGBError::Balance(e.to_string()))?;

        println!("Balance: {:?}", balance);

        Ok(balance.vanilla.spendable)
    }

    async fn list_unspents(&self) -> Result<Vec<Unspent>, ApplicationError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let unspents = wallet
            .list_unspents(Some(online), false)
            .map_err(|e| RGBError::Unspents(e.to_string()))?;

        Ok(unspents)
    }

    async fn send_btc(
        &self,
        address: String,
        amount: u64,
        fee_rate: f32,
    ) -> Result<String, ApplicationError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let tx_id = wallet
            .send_btc(online, address, amount, fee_rate)
            .map_err(|e| RGBError::Send(e.to_string()))?;

        Ok(tx_id)
    }

    async fn drain_btc(&self, address: String, fee_rate: f32) -> Result<String, ApplicationError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let tx_id = wallet
            .drain_to(online, address, true, fee_rate)
            .map_err(|e| RGBError::Send(e.to_string()))?;

        Ok(tx_id)
    }

    async fn create_utxos(&self, fee_rate: f32) -> Result<u8, ApplicationError> {
        let online = self.online().await?;

        let mut wallet = self.wallet.lock().await;

        let n = wallet
            .create_utxos(online, true, None, None, fee_rate)
            .map_err(|e| RGBError::Utxos(e.to_string()))?;

        println!("UTXOs created: {}", n);

        Ok(n)
    }

    async fn issue_contract(&self, contract: RGBContract) -> Result<String, ApplicationError> {
        let online = self.online().await?;

        let mut wallet = self.wallet.lock().await;

        let contract = wallet
            .issue_asset_nia(
                online,
                contract.ticker,
                contract.name,
                contract.precision,
                contract.amounts,
            )
            .map_err(|e| RGBError::ContractIssuance(e.to_string()))?;

        println!("Contract issued: {:?}", contract);

        Ok(contract.asset_id)
    }
}

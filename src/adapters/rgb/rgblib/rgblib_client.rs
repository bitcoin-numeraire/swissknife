use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use rgb_lib::{
    restore_keys,
    wallet::{
        Assets, Balance, DatabaseType, Metadata, Online, ReceiveData, Recipient, Unspent,
        WalletData,
    },
    Wallet,
};
use tokio::{sync::Mutex, task};

use crate::{
    adapters::rgb::RGBClient, application::errors::RGBError, domains::rgb::entities::RGBContract,
};

#[derive(Clone, Debug, Deserialize)]
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
    pub async fn new(config: RGBLibClientConfig) -> Result<Self, RGBError> {
        let keys = restore_keys(rgb_lib::BitcoinNetwork::Regtest, config.mnemonic)
            .map_err(|e| RGBError::RestoreKeys(e.to_string()))?;

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
            .map_err(|e| RGBError::CreateWallet(e.to_string()))?
            .map_err(|e| RGBError::CreateWallet(e.to_string()))?;

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
    async fn get_address(&self) -> Result<String, RGBError> {
        let wallet = self.wallet.lock().await;

        let address = wallet
            .get_address()
            .map_err(|e| RGBError::Address(e.to_string()))?;

        Ok(address)
    }

    async fn get_btc_balance(&self) -> Result<u64, RGBError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let balance = wallet
            .get_btc_balance(online)
            .map_err(|e| RGBError::Balance(e.to_string()))?;

        Ok(balance.vanilla.spendable)
    }

    async fn list_unspents(&self) -> Result<Vec<Unspent>, RGBError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let unspents = wallet
            .list_unspents(Some(online), false)
            .map_err(|e| RGBError::ListUnspents(e.to_string()))?;

        Ok(unspents)
    }

    async fn send_btc(
        &self,
        address: String,
        amount: u64,
        fee_rate: f32,
    ) -> Result<String, RGBError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let tx_id = wallet
            .send_btc(online, address, amount, fee_rate)
            .map_err(|e| RGBError::SendBTC(e.to_string()))?;

        Ok(tx_id)
    }

    async fn drain_btc(&self, address: String, fee_rate: f32) -> Result<String, RGBError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let tx_id = wallet
            .drain_to(online, address, true, fee_rate)
            .map_err(|e| RGBError::SendBTC(e.to_string()))?;

        Ok(tx_id)
    }

    async fn create_utxos(&self, fee_rate: f32) -> Result<u8, RGBError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let n = wallet
            .create_utxos(online, true, None, None, fee_rate)
            .map_err(|e| RGBError::CreateUtxos(e.to_string()))?;

        Ok(n)
    }

    async fn issue_contract(&self, contract: RGBContract) -> Result<String, RGBError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let contract = wallet
            .issue_asset_nia(
                online,
                contract.ticker,
                contract.name,
                contract.precision,
                contract.amounts,
            )
            .map_err(|e| RGBError::ContractIssuance(e.to_string()))?;

        Ok(contract.asset_id)
    }

    async fn list_assets(&self) -> Result<Assets, RGBError> {
        let wallet = self.wallet.lock().await;

        let assets = wallet
            .list_assets(Vec::new())
            .map_err(|e| RGBError::ListAssets(e.to_string()))?;

        Ok(assets)
    }

    async fn get_asset(&self, asset_id: String) -> Result<Metadata, RGBError> {
        let wallet = self.wallet.lock().await;

        let asset = wallet
            .get_asset_metadata(asset_id)
            .map_err(|e| RGBError::GetAsset(e.to_string()))?;

        Ok(asset)
    }

    async fn get_asset_balance(&self, asset_id: String) -> Result<Balance, RGBError> {
        let wallet = self.wallet.lock().await;

        let asset = wallet
            .get_asset_balance(asset_id)
            .map_err(|e| RGBError::GetAssetBalance(e.to_string()))?;

        Ok(asset)
    }

    async fn send(
        &self,
        asset_id: String,
        recipients: Vec<Recipient>,
        donation: bool,
        fee_rate: f32,
        min_confirmations: u8,
    ) -> Result<String, RGBError> {
        let online = self.online().await?;

        let wallet = self.wallet.lock().await;

        let mut recipient_map: HashMap<String, Vec<Recipient>> = HashMap::new();
        recipient_map.insert(asset_id, recipients);

        let tx_id = wallet
            .send(online, recipient_map, donation, fee_rate, min_confirmations)
            .map_err(|e| RGBError::Send(e.to_string()))?;

        Ok(tx_id)
    }

    async fn invoice(
        &self,
        asset_id: Option<String>,
        amount: Option<u64>,
        duration_seconds: Option<u32>,
        transport_endpoints: Vec<String>,
        min_confirmations: u8,
    ) -> Result<ReceiveData, RGBError> {
        let wallet = self.wallet.lock().await;

        let invoice = wallet
            .blind_receive(
                asset_id,
                amount,
                duration_seconds,
                transport_endpoints,
                min_confirmations,
            )
            .map_err(|e| RGBError::Invoice(e.to_string()))?;

        Ok(invoice)
    }
}

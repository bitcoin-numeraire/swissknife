use serde::Deserialize;
use std::{collections::HashMap, str::FromStr, sync::Arc};

use async_trait::async_trait;
use rgb_lib::{
    restore_keys,
    wallet::{
        Assets, Balance, DatabaseType, Metadata, Online, ReceiveData, Recipient, RecipientData,
        SendResult, Transfer, Unspent, WalletData,
    },
    BitcoinNetwork, SecretSeal, Wallet,
};
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::{
    application::errors::RGBError, domains::rgb::entities::RGBAsset, infra::rgb::RGBClient,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RGBLibClientConfig {
    pub data_dir: String,
    pub mnemonic: String,
    pub electrum_url: String,
    pub media_dir: String,
    pub min_confirmations: u8,
    pub proxy_server_url: String,
}

pub struct RGBLibClient {
    wallet: Arc<Mutex<Wallet>>,
    media_dir: String,
    min_confirmations: u8,
    transport_endpoints: Vec<String>,
    online: Online,
}

impl RGBLibClient {
    pub async fn new(config: RGBLibClientConfig) -> Result<Self, RGBError> {
        let keys = restore_keys(BitcoinNetwork::Regtest, config.mnemonic)
            .map_err(|e| RGBError::RestoreKeys(e.to_string()))?;

        let wallet_data = WalletData {
            bitcoin_network: BitcoinNetwork::Regtest,
            database_type: DatabaseType::Sqlite,
            data_dir: config.data_dir,
            pubkey: keys.account_xpub,
            mnemonic: Some(keys.mnemonic),
            vanilla_keychain: None,
            max_allocations_per_utxo: 5,
        };

        // Offload the blocking Wallet::new call to a separate thread
        let mut wallet = spawn_blocking(move || Wallet::new(wallet_data))
            .await
            .map_err(|e| RGBError::CreateWallet(e.to_string()))?
            .map_err(|e| RGBError::CreateWallet(e.to_string()))?;

        let online = wallet
            .go_online(false, config.electrum_url.clone())
            .map_err(|e| RGBError::Online(e.to_string()))?;

        Ok(Self {
            wallet: Arc::new(Mutex::new(wallet)),
            media_dir: config.media_dir,
            min_confirmations: config.min_confirmations,
            transport_endpoints: vec![config.proxy_server_url],
            online,
        })
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
        let wallet = self.wallet.lock().await;

        let balance = wallet
            .get_btc_balance(self.online.clone())
            .map_err(|e| RGBError::Balance(e.to_string()))?;

        Ok(balance.vanilla.spendable)
    }

    async fn list_unspents(&self) -> Result<Vec<Unspent>, RGBError> {
        let wallet = self.wallet.lock().await;

        let unspents = wallet
            .list_unspents(Some(self.online.clone()), false)
            .map_err(|e| RGBError::ListUnspents(e.to_string()))?;

        Ok(unspents)
    }

    async fn send_btc(
        &self,
        address: String,
        amount: u64,
        fee_rate: f32,
    ) -> Result<String, RGBError> {
        let wallet = self.wallet.lock().await;

        let tx_id = wallet
            .send_btc(self.online.clone(), address, amount, fee_rate)
            .map_err(|e| RGBError::SendBTC(e.to_string()))?;

        Ok(tx_id)
    }

    async fn drain_btc(&self, address: String, fee_rate: f32) -> Result<String, RGBError> {
        let wallet = self.wallet.lock().await;

        let tx_id = wallet
            .drain_to(self.online.clone(), address, true, fee_rate)
            .map_err(|e| RGBError::SendBTC(e.to_string()))?;

        Ok(tx_id)
    }

    async fn create_utxos(&self, fee_rate: f32) -> Result<u8, RGBError> {
        let wallet = self.wallet.lock().await;

        let n = wallet
            .create_utxos(self.online.clone(), true, None, None, fee_rate)
            .map_err(|e| RGBError::CreateUtxos(e.to_string()))?;

        Ok(n)
    }

    async fn issue_asset_nia(&self, contract: RGBAsset) -> Result<String, RGBError> {
        let wallet = self.wallet.lock().await;

        let asset = wallet
            .issue_asset_nia(
                self.online.clone(),
                contract.ticker,
                contract.name,
                contract.precision,
                contract.amounts,
            )
            .map_err(|e| RGBError::ContractIssuanceNIA(e.to_string()))?;

        Ok(asset.asset_id)
    }

    async fn issue_asset_cfa(&self, contract: RGBAsset) -> Result<String, RGBError> {
        let wallet = self.wallet.lock().await;

        let media_file_path = contract
            .filename
            .as_ref()
            .map(|filename| format!("{}/{}", self.media_dir, filename));

        let asset = wallet
            .issue_asset_cfa(
                self.online.clone(),
                contract.name,
                contract.details,
                contract.precision,
                contract.amounts,
                media_file_path,
            )
            .map_err(|e| RGBError::ContractIssuanceCFA(e.to_string()))?;

        Ok(asset.asset_id)
    }

    async fn issue_asset_uda(&self, contract: RGBAsset) -> Result<String, RGBError> {
        let wallet = self.wallet.lock().await;

        let media_file_path = contract
            .filename
            .as_ref()
            .map(|filename| format!("{}/{}", self.media_dir, filename));

        let asset = wallet
            .issue_asset_uda(
                self.online.clone(),
                contract.ticker,
                contract.name,
                contract.details,
                contract.precision,
                media_file_path,
                vec![],
            )
            .map_err(|e| RGBError::ContractIssuanceUDA(e.to_string()))?;

        Ok(asset.asset_id)
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
        recipient_id: String,
        donation: bool,
        fee_rate: f32,
        amount: u64,
    ) -> Result<SendResult, RGBError> {
        let secret_seal = SecretSeal::from_str(&recipient_id)
            .map_err(|e| RGBError::InvalidRecipient(e.to_string()))?;

        let recipient_map = HashMap::from([(
            asset_id.clone(),
            vec![Recipient {
                recipient_data: RecipientData::BlindedUTXO(secret_seal),
                amount,
                transport_endpoints: self.transport_endpoints.clone(),
            }],
        )]);

        let wallet = self.wallet.clone();
        let min_confirmations = self.min_confirmations;
        let online = self.online.clone();

        let result = spawn_blocking(move || {
            wallet.blocking_lock().send(
                online,
                recipient_map,
                donation,
                fee_rate,
                min_confirmations,
            )
        })
        .await
        .map_err(|e| RGBError::Send(e.to_string()))?
        .map_err(|e| RGBError::Send(e.to_string()))?;

        Ok(result)
    }

    async fn blind_receive(
        &self,
        asset_id: Option<String>,
        amount: Option<u64>,
        duration_seconds: Option<u32>,
    ) -> Result<ReceiveData, RGBError> {
        let wallet = self.wallet.lock().await;

        let invoice = wallet
            .blind_receive(
                asset_id,
                amount,
                duration_seconds,
                self.transport_endpoints.clone(),
                self.min_confirmations,
            )
            .map_err(|e| RGBError::Invoice(e.to_string()))?;

        Ok(invoice)
    }

    async fn witness_receive(
        &self,
        asset_id: Option<String>,
        amount: Option<u64>,
        duration_seconds: Option<u32>,
    ) -> Result<ReceiveData, RGBError> {
        let wallet = self.wallet.lock().await;

        let invoice: ReceiveData = wallet
            .witness_receive(
                asset_id,
                amount,
                duration_seconds,
                self.transport_endpoints.clone(),
                self.min_confirmations,
            )
            .map_err(|e| RGBError::Invoice(e.to_string()))?;

        Ok(invoice)
    }

    async fn list_transfers(&self, asset_id: Option<String>) -> Result<Vec<Transfer>, RGBError> {
        let wallet = self.wallet.lock().await;

        let transfers = wallet
            .list_transfers(asset_id)
            .map_err(|e| RGBError::ListTransfers(e.to_string()))?;

        Ok(transfers)
    }

    async fn refresh(&self, asset_id: Option<String>) -> Result<(), RGBError> {
        let wallet = self.wallet.clone();
        let online = self.online.clone();

        spawn_blocking(move || wallet.blocking_lock().refresh(online, asset_id, vec![]))
            .await
            .map_err(|e| RGBError::Refresh(e.to_string()))?
            .map_err(|e| RGBError::Refresh(e.to_string()))?;

        Ok(())
    }
}

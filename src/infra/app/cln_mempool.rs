use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
    time::Duration,
};

use bitcoin::{address::NetworkUnchecked, consensus, Address, Network, ScriptBuf, Transaction, Txid};
use reqwest::{redirect::Policy, Client, Url};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use tokio::time::sleep;
use tracing::warn;

use crate::{
    application::{
        composition::AppServices,
        errors::{ApplicationError, BitcoinError, ConfigError},
    },
    domains::{bitcoin::BtcNetwork, event::OnchainDepositEvent},
};

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const RPC_TIMEOUT: Duration = Duration::from_secs(20);
const TRANSACTION_BATCH_SIZE: usize = 64;

#[derive(Clone, Deserialize)]
pub struct ClnMempoolConfig {
    pub rpc_url: String,
    pub rpc_user: String,
    pub rpc_password: String,
}

impl fmt::Debug for ClnMempoolConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("ClnMempoolConfig").finish_non_exhaustive()
    }
}

pub struct ClnMempool {
    client: Client,
    endpoint: Url,
    config: ClnMempoolConfig,
    network: Network,
    services: Arc<AppServices>,
}

impl ClnMempool {
    pub fn new(config: ClnMempoolConfig, network: BtcNetwork, services: Arc<AppServices>) -> Result<Self, ConfigError> {
        let endpoint = Url::parse(&config.rpc_url)
            .ok()
            .filter(|url| {
                matches!(url.scheme(), "http" | "https") && url.username().is_empty() && url.password().is_none()
            })
            .ok_or_else(|| ConfigError::Load("CLN mempool RPC requires an HTTP(S) URL without credentials".into()))?;
        let network = match network {
            BtcNetwork::Bitcoin => Network::Bitcoin,
            BtcNetwork::Testnet => Network::Testnet,
            BtcNetwork::Testnet4 => Network::Testnet4,
            BtcNetwork::Regtest => Network::Regtest,
            BtcNetwork::Signet => Network::Signet,
            BtcNetwork::Simnet => return Err(ConfigError::Load("Bitcoin Core does not support simnet".into())),
        };
        let client = Client::builder()
            .redirect(Policy::none())
            .timeout(RPC_TIMEOUT)
            .build()
            .map_err(|err| ConfigError::Load(err.to_string()))?;

        Ok(Self {
            client,
            endpoint,
            config,
            network,
            services,
        })
    }

    pub fn start(self) {
        tokio::spawn(async move {
            let mut seen = HashSet::new();
            let mut network_verified = false;
            loop {
                if !network_verified {
                    match self.verify_network().await {
                        Ok(()) => network_verified = true,
                        Err(err) => {
                            warn!(%err, "Cannot verify the Bitcoin Core mempool source; retrying");
                            sleep(POLL_INTERVAL).await;
                            continue;
                        }
                    }
                }

                if let Err(err) = self.scan(&mut seen).await {
                    warn!(%err, "CLN mempool scan failed; retrying");
                    network_verified = false;
                }

                sleep(POLL_INTERVAL).await;
            }
        });
    }

    async fn verify_network(&self) -> Result<(), BitcoinError> {
        #[derive(Deserialize)]
        struct BlockchainInfo {
            chain: String,
        }

        let response: RpcResponse<BlockchainInfo> = self.request(&rpc("getblockchaininfo", json!([]), "chain")).await?;
        let chain = response.result()?.chain;
        if chain != self.network.to_core_arg() {
            return Err(BitcoinError::Synchronize(format!(
                "Bitcoin Core reports {chain}, expected {}",
                self.network.to_core_arg()
            )));
        }
        Ok(())
    }

    async fn scan(&self, seen: &mut HashSet<Txid>) -> Result<(), ApplicationError> {
        let response: RpcResponse<HashSet<Txid>> = self.request(&rpc("getrawmempool", json!([]), "mempool")).await?;
        let mempool = response.result()?;
        seen.retain(|txid| mempool.contains(txid));
        let new_transactions: Vec<_> = mempool.difference(seen).copied().collect();
        if new_transactions.is_empty() {
            return Ok(());
        }

        // Read registered addresses after taking the mempool snapshot so that
        // newly issued addresses are included when their first deposit arrives.
        let addresses = self.services.bitcoin.list_addresses(Default::default()).await?;
        let scripts: HashMap<ScriptBuf, String> = addresses
            .into_iter()
            .filter_map(|address| {
                let script = address
                    .address
                    .parse::<Address<NetworkUnchecked>>()
                    .ok()?
                    .require_network(self.network)
                    .ok()?;
                Some((script.script_pubkey(), address.address))
            })
            .collect();
        if scripts.is_empty() {
            return Ok(());
        }

        for txids in new_transactions.chunks(TRANSACTION_BATCH_SIZE) {
            let requests: Vec<_> = txids
                .iter()
                .map(|txid| rpc("getrawtransaction", json!([txid]), txid.to_string()))
                .collect();
            let responses: Vec<TransactionResponse> = self.request(&json!(requests)).await?;
            for TransactionResponse { id, response } in responses {
                // A transaction may leave the mempool between the snapshot and this fetch.
                if response.error.as_ref().is_some_and(|error| error.code == -5) {
                    continue;
                }

                let raw = hex::decode(response.result()?).map_err(|err| BitcoinError::Synchronize(err.to_string()))?;
                let transaction: Transaction =
                    consensus::deserialize(&raw).map_err(|err| BitcoinError::Synchronize(err.to_string()))?;
                if transaction.compute_txid() != id || !txids.contains(&id) {
                    return Err(
                        BitcoinError::Synchronize("Bitcoin Core returned an unexpected transaction".into()).into(),
                    );
                }

                for (index, output) in transaction.output.iter().enumerate() {
                    if let Some(address) = scripts.get(&output.script_pubkey) {
                        self.services
                            .event
                            .onchain_deposit(OnchainDepositEvent {
                                txid: id.to_string(),
                                output_index: index as u32,
                                address: address.clone(),
                                amount_sat: output.value.to_sat(),
                                block_height: None,
                            })
                            .await?;
                    }
                }
                seen.insert(id);
            }
        }
        Ok(())
    }

    async fn request<T: DeserializeOwned>(&self, body: &Value) -> Result<T, BitcoinError> {
        let response = self
            .client
            .post(self.endpoint.clone())
            .basic_auth(&self.config.rpc_user, Some(&self.config.rpc_password))
            .json(body)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|err| BitcoinError::Synchronize(err.without_url().to_string()))?;
        response
            .json()
            .await
            .map_err(|err| BitcoinError::Synchronize(err.without_url().to_string()))
    }
}

fn rpc(method: &str, params: Value, id: impl serde::Serialize) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

impl<T> RpcResponse<T> {
    fn result(self) -> Result<T, BitcoinError> {
        self.result.ok_or_else(|| {
            let message = self.error.map_or_else(
                || "Bitcoin Core RPC response is missing a result".into(),
                |error| format!("Bitcoin Core RPC returned error {}", error.code),
            );
            BitcoinError::Synchronize(message)
        })
    }
}

#[derive(Deserialize)]
struct RpcError {
    code: i32,
}

#[derive(Deserialize)]
struct TransactionResponse {
    id: Txid,
    #[serde(flatten)]
    response: RpcResponse<String>,
}

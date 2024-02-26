use thiserror::Error;

#[derive(Debug, Error)]
pub enum RGBError {
    #[error("Failed to restore keys with mnemonic and network: {0}")]
    RestoreKeys(String),

    #[error("Failed to create wallet: {0}")]
    CreateWallet(String),

    #[error("Failed to get address: {0}")]
    Address(String),

    #[error("Failed to get bitcoin balance: {0}")]
    Balance(String),

    #[error("Failed to list UTXOs: {0}")]
    ListUnspents(String),

    #[error("Failed to create UTXOs: {0}")]
    CreateUtxos(String),

    #[error("Failed to issue CFA smart contract: {0}")]
    ContractIssuanceCFA(String),

    #[error("Failed to issue NIA smart contract: {0}")]
    ContractIssuanceNIA(String),

    #[error("Failed to issue UDA smart contract: {0}")]
    ContractIssuanceUDA(String),

    #[error("Failed to connect to node: {0}")]
    Online(String),

    #[error("Failed to send bitcoins: {0}")]
    SendBTC(String),

    #[error("Failed to list RGB assets: {0}")]
    ListAssets(String),

    #[error("Failed to list RGB transfers: {0}")]
    ListTransfers(String),

    #[error("Failed to get RGB asset: {0}")]
    GetAsset(String),

    #[error("Failed to get RGB asset balance: {0}")]
    GetAssetBalance(String),

    #[error("Invalid Recipient ID: {0}")]
    InvalidRecipient(String),

    #[error("Failed to send RGB asset: {0}")]
    Send(String),

    #[error("Failed to generate RGB invoice: {0}")]
    Invoice(String),

    #[error("Failed to refresh RGB transfers: {0}")]
    Refresh(String),
}

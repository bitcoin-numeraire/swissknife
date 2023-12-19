#[derive(Debug)]
pub enum RGBError {
    RestoreKeys(String),
    CreateWallet(String),
    Address(String),
    Balance(String),
    Unspents(String),
    Utxos(String),
    ContractIssuance(String),
    Online(String),
    SendBTC(String),
    ListAssets(String),
    GetAsset(String),
    GetAssetBalance(String),
    Send(String),
    Invoice(String),
}

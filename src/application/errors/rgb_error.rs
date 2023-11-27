#[derive(Debug)]
pub enum RGBError {
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

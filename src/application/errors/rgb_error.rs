#[derive(Debug)]
pub enum RGBError {
    Address(String),
    Balance(String),
    Utxos(String),
    ContractIssuance(String),
    Online(String),
    Send(String),
}

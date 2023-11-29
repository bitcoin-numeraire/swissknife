#[derive(Debug)]
pub enum LightningError {
    Invoice(String),
    NodeInfo(String),
    ListPayments(String),
}

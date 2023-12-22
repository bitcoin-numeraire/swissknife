#[derive(Debug)]
pub enum LightningError {
    Seed(String),
    Connect(String),
    Invoice(String),
    NodeInfo(String),
    ListPayments(String),
}

/// Options for creating a Lightning invoice with the configured provider.
#[derive(Default)]
pub struct LnInvoiceOptions {
    pub description_hash_only: bool,
    /// Set when SwissKnife needs to prove settlement even for an internal transfer.
    pub preimage: Option<[u8; 32]>,
}

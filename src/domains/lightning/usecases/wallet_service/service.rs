use crate::domains::lightning::adapters::LightningRepository;

pub struct WalletService {
    pub store: Box<dyn LightningRepository>,
}

impl WalletService {
    pub fn new(store: Box<dyn LightningRepository>) -> Self {
        WalletService { store }
    }
}

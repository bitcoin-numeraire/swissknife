#[derive(Clone, Debug)]
pub struct BitcoinBalance {
    pub confirmed_sat: u64,
    pub unconfirmed_sat: u64,
}

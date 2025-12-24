#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BitcoinBalance {
    pub confirmed_sat: u64,
    pub unconfirmed_sat: u64,
}

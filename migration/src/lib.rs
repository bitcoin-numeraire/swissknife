pub use sea_orm_migration::prelude::*;

mod m20240420_1_wallet_table;
mod m20240420_2_ln_address_table;
mod m20240420_3_invoice_table;
mod m20240420_4_payment_table;
mod m20241005_5_ln_address_nostr;
mod m20241009_6_api_key_table;
mod m20241028_135908_permissions_as_json;
mod m20241028_154716_dates_without_tz;
mod m20250106_141600_config_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240420_1_wallet_table::Migration),
            Box::new(m20240420_2_ln_address_table::Migration),
            Box::new(m20240420_3_invoice_table::Migration),
            Box::new(m20240420_4_payment_table::Migration),
            Box::new(m20241005_5_ln_address_nostr::Migration),
            Box::new(m20241009_6_api_key_table::Migration),
            Box::new(m20241028_135908_permissions_as_json::Migration),
            Box::new(m20241028_154716_dates_without_tz::Migration),
            Box::new(m20250106_141600_config_table::Migration),
        ]
    }
}

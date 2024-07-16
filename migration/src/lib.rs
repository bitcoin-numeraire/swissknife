pub use sea_orm_migration::prelude::*;

mod m20240420_1_account_table;
mod m20240420_2_wallet_table;
mod m20240420_3_ln_address_table;
mod m20240420_4_invoice_table;
mod m20240420_5_payment_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240420_1_account_table::Migration),
            Box::new(m20240420_2_wallet_table::Migration),
            Box::new(m20240420_3_ln_address_table::Migration),
            Box::new(m20240420_4_invoice_table::Migration),
            Box::new(m20240420_5_payment_table::Migration),
        ]
    }
}

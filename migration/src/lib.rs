pub use sea_orm_migration::prelude::*;

mod m20240420_194334_lightning_addresses_table;
mod m20240420_195225_lightning_invoices_table;
mod m20240420_195439_lightning_payments_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240420_194334_lightning_addresses_table::Migration),
            Box::new(m20240420_195225_lightning_invoices_table::Migration),
            Box::new(m20240420_195439_lightning_payments_table::Migration),
        ]
    }
}

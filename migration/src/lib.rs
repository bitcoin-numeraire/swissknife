pub use sea_orm_migration::prelude::*;

mod m20240420_194334_lightning_address_table;
mod m20240420_195225_lightning_invoice_table;
mod m20240420_195439_lightning_payment_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240420_194334_lightning_address_table::Migration),
            Box::new(m20240420_195225_lightning_invoice_table::Migration),
            Box::new(m20240420_195439_lightning_payment_table::Migration),
        ]
    }
}

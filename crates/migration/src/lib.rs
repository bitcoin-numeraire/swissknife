pub use sea_orm_migration::prelude::*;

mod m20240420_000001_wallet_table;
mod m20240420_000002_ln_address_table;
mod m20240420_000003_invoice_table;
mod m20240420_000004_payment_table;
mod m20241005_000005_ln_address_nostr;
mod m20241009_000006_api_key_table;
mod m20241028_135908_permissions_as_json;
mod m20250106_141600_config_table;
mod m20251021_162217_convert_timestamptz_to_timestamp;
mod m20251224_162538_btc_address_table;
mod m20251224_162542_btc_output_table;
mod m20251224_162546_btc_fields_to_invoice;
mod m20251224_162550_btc_fields_to_payment;
mod m20260113_222755_fix_invoice_payment_hash_unique;
mod m20260609_143600_wallet_balance_table;
mod m20260609_143601_backfill_wallet_balances;
mod m20260704_000001_account_table;
mod m20260704_000002_auth_identity_table;
mod m20260704_000004_account_preference_table;
mod m20260704_000005_asset_table;
mod m20260704_000006_api_key_account_id;
mod m20260704_000007_backfill_oauth2_accounts;
mod m20260704_000008_wallet_account_asset_schema;
mod m20260704_000009_backfill_mainnet_wallet_accounts;
mod m20260704_000010_finalize_wallet_schema;
mod m20260704_000011_ln_address_account_routes;
mod m20260704_000012_drop_legacy_wallet_contract;
mod m20260710_234825_add_relationship_indexes;
mod m20260717_105719_persist_lnurl_success_action;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240420_000001_wallet_table::Migration),
            Box::new(m20240420_000002_ln_address_table::Migration),
            Box::new(m20240420_000003_invoice_table::Migration),
            Box::new(m20240420_000004_payment_table::Migration),
            Box::new(m20241005_000005_ln_address_nostr::Migration),
            Box::new(m20241009_000006_api_key_table::Migration),
            Box::new(m20241028_135908_permissions_as_json::Migration),
            Box::new(m20250106_141600_config_table::Migration),
            Box::new(m20251021_162217_convert_timestamptz_to_timestamp::Migration),
            Box::new(m20251224_162538_btc_address_table::Migration),
            Box::new(m20251224_162542_btc_output_table::Migration),
            Box::new(m20251224_162546_btc_fields_to_invoice::Migration),
            Box::new(m20251224_162550_btc_fields_to_payment::Migration),
            Box::new(m20260113_222755_fix_invoice_payment_hash_unique::Migration),
            Box::new(m20260609_143600_wallet_balance_table::Migration),
            Box::new(m20260609_143601_backfill_wallet_balances::Migration),
            Box::new(m20260704_000001_account_table::Migration),
            Box::new(m20260704_000002_auth_identity_table::Migration),
            Box::new(m20260704_000004_account_preference_table::Migration),
            Box::new(m20260704_000005_asset_table::Migration),
            Box::new(m20260704_000006_api_key_account_id::Migration),
            Box::new(m20260704_000007_backfill_oauth2_accounts::Migration),
            Box::new(m20260704_000008_wallet_account_asset_schema::Migration),
            Box::new(m20260704_000009_backfill_mainnet_wallet_accounts::Migration),
            Box::new(m20260704_000010_finalize_wallet_schema::Migration),
            Box::new(m20260704_000011_ln_address_account_routes::Migration),
            Box::new(m20260704_000012_drop_legacy_wallet_contract::Migration),
            Box::new(m20260710_234825_add_relationship_indexes::Migration),
            Box::new(m20260717_105719_persist_lnurl_success_action::Migration),
        ]
    }
}

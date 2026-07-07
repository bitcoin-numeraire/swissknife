use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const OAUTH2_PROVIDER: &str = "oauth2";
const BTC_MAINNET: &str = "bitcoin/mainnet";
const NATIVE_ASSET_REF: &str = "native";
const LEGACY_BTC_CURRENCY: &str = "Bitcoin";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        execute(
            db,
            format!(
                r#"
                UPDATE wallet
                SET account_id = (
                        SELECT account_id
                        FROM auth_identity
                        WHERE provider = '{provider}'
                          AND subject = wallet.user_id
                    ),
                    asset_id = (
                        SELECT id
                        FROM asset
                        WHERE protocol = 'bitcoin'
                          AND network = '{network}'
                          AND asset_ref = '{asset_ref}'
                    ),
                    available_amount = COALESCE((
                        SELECT available_amount
                        FROM wallet_balance
                        WHERE wallet_balance.wallet_id = wallet.id
                          AND wallet_balance.currency = '{currency}'
                    ), 0),
                    reserved_amount = COALESCE((
                        SELECT reserved_amount
                        FROM wallet_balance
                        WHERE wallet_balance.wallet_id = wallet.id
                          AND wallet_balance.currency = '{currency}'
                    ), 0),
                    updated_at = CURRENT_TIMESTAMP
                "#,
                provider = OAUTH2_PROVIDER,
                network = BTC_MAINNET,
                asset_ref = NATIVE_ASSET_REF,
                currency = LEGACY_BTC_CURRENCY,
            ),
        )
        .await?;

        assert_wallets_backfilled(db).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        execute(
            manager.get_connection(),
            r#"
            UPDATE wallet
            SET account_id = NULL,
                asset_id = NULL,
                available_amount = 0,
                reserved_amount = 0,
                updated_at = CURRENT_TIMESTAMP
            "#
            .to_string(),
        )
        .await
    }
}

async fn assert_wallets_backfilled(db: &dyn ConnectionTrait) -> Result<(), DbErr> {
    let missing = db
        .query_one(statement(
            db,
            "SELECT COUNT(*) AS count FROM wallet WHERE account_id IS NULL OR asset_id IS NULL".to_string(),
        ))
        .await?
        .ok_or_else(|| DbErr::Migration("wallet backfill count returned no row".to_string()))?
        .try_get::<i64>("", "count")?;

    if missing > 0 {
        return Err(DbErr::Migration(format!(
            "wallet account/asset backfill left {missing} wallet rows without account_id or asset_id"
        )));
    }

    Ok(())
}

async fn execute(db: &dyn ConnectionTrait, sql: String) -> Result<(), DbErr> {
    db.execute(statement(db, sql)).await.map(|_| ())
}

fn statement(db: &dyn ConnectionTrait, sql: String) -> Statement {
    Statement::from_string(db.get_database_backend(), sql)
}

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use sea_orm_migration::{prelude::*, schema::*};
use uuid::Uuid;

#[derive(DeriveMigrationName)]
pub struct Migration;

const ASSETS: &[SeedAsset] = &[
    SeedAsset {
        id: "00000000-0000-4000-8000-000000000001",
        code: "BTC",
        name: "Bitcoin",
        protocol: "bitcoin",
        network: "bitcoin/mainnet",
        asset_ref: "native",
        display_ticker: "BTC",
        decimals: 11,
    },
    SeedAsset {
        id: "00000000-0000-4000-8000-000000000002",
        code: "BTC",
        name: "Bitcoin testnet",
        protocol: "bitcoin",
        network: "bitcoin/testnet",
        asset_ref: "native",
        display_ticker: "tBTC",
        decimals: 11,
    },
    SeedAsset {
        id: "00000000-0000-4000-8000-000000000003",
        code: "BTC",
        name: "Bitcoin testnet4",
        protocol: "bitcoin",
        network: "bitcoin/testnet4",
        asset_ref: "native",
        display_ticker: "tBTC",
        decimals: 11,
    },
    SeedAsset {
        id: "00000000-0000-4000-8000-000000000004",
        code: "BTC",
        name: "Bitcoin regtest",
        protocol: "bitcoin",
        network: "bitcoin/regtest",
        asset_ref: "native",
        display_ticker: "rBTC",
        decimals: 11,
    },
    SeedAsset {
        id: "00000000-0000-4000-8000-000000000005",
        code: "BTC",
        name: "Bitcoin simnet",
        protocol: "bitcoin",
        network: "bitcoin/simnet",
        asset_ref: "native",
        display_ticker: "sBTC",
        decimals: 11,
    },
    SeedAsset {
        id: "00000000-0000-4000-8000-000000000006",
        code: "BTC",
        name: "Bitcoin signet",
        protocol: "bitcoin",
        network: "bitcoin/signet",
        asset_ref: "native",
        display_ticker: "sBTC",
        decimals: 11,
    },
];

struct SeedAsset {
    id: &'static str,
    code: &'static str,
    name: &'static str,
    protocol: &'static str,
    network: &'static str,
    asset_ref: &'static str,
    display_ticker: &'static str,
    decimals: i16,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Asset::Table)
                    .if_not_exists()
                    .col(uuid(Asset::Id).primary_key())
                    .col(string_len(Asset::Code, 255))
                    .col(text_null(Asset::Name))
                    .col(string_len(Asset::Protocol, 255))
                    .col(string_len(Asset::Network, 255))
                    .col(string_len(Asset::AssetRef, 255))
                    .col(string_len(Asset::DisplayTicker, 255))
                    .col(small_integer(Asset::Decimals))
                    .col(timestamp(Asset::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp_null(Asset::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_asset_protocol_network_ref")
                    .table(Asset::Table)
                    .col(Asset::Protocol)
                    .col(Asset::Network)
                    .col(Asset::AssetRef)
                    .unique()
                    .to_owned(),
            )
            .await?;

        seed_assets(manager.get_connection()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_asset_protocol_network_ref")
                    .table(Asset::Table)
                    .to_owned(),
            )
            .await?;

        manager.drop_table(Table::drop().table(Asset::Table).to_owned()).await
    }
}

async fn seed_assets(db: &dyn ConnectionTrait) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    for asset in ASSETS {
        let id = uuid_literal(backend, asset.id)?;
        let sql = format!(
            r#"
            INSERT INTO asset (
                id,
                code,
                name,
                protocol,
                network,
                asset_ref,
                display_ticker,
                decimals,
                created_at
            )
            SELECT
                {id},
                '{code}',
                '{name}',
                '{protocol}',
                '{network}',
                '{asset_ref}',
                '{display_ticker}',
                {decimals},
                CURRENT_TIMESTAMP
            WHERE NOT EXISTS (
                SELECT 1
                FROM asset
                WHERE protocol = '{protocol}'
                  AND network = '{network}'
                  AND asset_ref = '{asset_ref}'
            )
            "#,
            id = id,
            code = sql_literal(asset.code),
            name = sql_literal(asset.name),
            protocol = sql_literal(asset.protocol),
            network = sql_literal(asset.network),
            asset_ref = sql_literal(asset.asset_ref),
            display_ticker = sql_literal(asset.display_ticker),
            decimals = asset.decimals,
        );
        execute(db, backend, sql).await?;
    }

    Ok(())
}

async fn execute(db: &dyn ConnectionTrait, backend: DatabaseBackend, sql: String) -> Result<(), DbErr> {
    db.execute(Statement::from_string(backend, sql)).await?;
    Ok(())
}

fn sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn uuid_literal(backend: DatabaseBackend, value: &str) -> Result<String, DbErr> {
    let uuid =
        Uuid::parse_str(value).map_err(|err| DbErr::Migration(format!("invalid UUID literal {value}: {err}")))?;

    Ok(match backend {
        DatabaseBackend::Sqlite => format!("X'{}'", uuid.simple()),
        _ => format!("'{uuid}'"),
    })
}

#[derive(DeriveIden)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Asset {
    Table,
    Id,
    Code,
    Name,
    Protocol,
    Network,
    AssetRef,
    DisplayTicker,
    Decimals,
    CreatedAt,
    UpdatedAt,
}

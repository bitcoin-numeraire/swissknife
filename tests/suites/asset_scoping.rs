//! Multi-asset wallet isolation for one account. These assertions fail if
//! balances, resources, or overview aggregates are keyed by account or ticker
//! instead of the concrete wallet and asset network.

use reqwest::StatusCode;

use swissknife_types::{
    Balance, BtcNetwork, CreateWalletRequest, Invoice, NewInvoiceRequest, SendPaymentRequest, Wallet, WalletOverview,
};

use crate::common::fixtures::{mainnet_btc_asset_id, signet_btc_asset_id};
use crate::common::{app, assert_error, assert_status, Auth};

#[tokio::test]
async fn keeps_balances_resources_and_aggregates_separate_across_bitcoin_networks() {
    let app = app().await;
    let admin = app.admin_token().await;
    let account = app.create_account_with_wallet(admin, "asset-scope").await;
    let regtest = account.wallet;

    let mainnet = create_wallet(app, &account.key, mainnet_btc_asset_id()).await;
    let signet = create_wallet(app, &account.key, signet_btc_asset_id()).await;
    let mainnet_again = create_wallet(app, &account.key, mainnet_btc_asset_id()).await;
    assert_eq!(mainnet_again.id, mainnet.id, "one wallet exists per account and asset");

    let wallets = app
        .api()
        .get("/v1/me/wallets?limit=1000", Auth::ApiKey(&account.key))
        .await;
    assert_status(&wallets, StatusCode::OK);
    let wallets = wallets.parse::<Vec<Wallet>>();
    assert_eq!(
        wallets
            .iter()
            .filter(|wallet| wallet.account_id == account.account.id)
            .count(),
        3
    );
    assert_network(&wallets, regtest.id, BtcNetwork::Regtest);
    assert_network(&wallets, mainnet.id, BtcNetwork::Bitcoin);
    assert_network(&wallets, signet.id, BtcNetwork::Signet);

    app.fund_onchain(admin, regtest.id, 100_000).await;
    let regtest_balance = balance(app, &account.key, regtest.id).await;
    let mainnet_balance = balance(app, &account.key, mainnet.id).await;
    let signet_balance = balance(app, &account.key, signet.id).await;
    assert!(regtest_balance.available_msat >= 100_000_000);
    assert_eq!(mainnet_balance.available_msat, 0);
    assert_eq!(signet_balance.available_msat, 0);

    let invoice = app
        .api()
        .post(
            &format!("/v1/me/wallets/{}/invoices", regtest.id),
            Auth::ApiKey(&account.key),
            NewInvoiceRequest {
                wallet_id: None,
                amount_msat: 25_000,
                description: Some("network-scoped invoice".to_string()),
                expiry: None,
            },
        )
        .await;
    assert_status(&invoice, StatusCode::OK);
    let invoice = invoice.parse::<Invoice>();

    let regtest_invoices = app
        .api()
        .get(
            &format!("/v1/me/wallets/{}/invoices", regtest.id),
            Auth::ApiKey(&account.key),
        )
        .await;
    assert!(regtest_invoices
        .parse::<Vec<Invoice>>()
        .iter()
        .any(|candidate| candidate.id == invoice.id));
    let mainnet_invoices = app
        .api()
        .get(
            &format!("/v1/me/wallets/{}/invoices", mainnet.id),
            Auth::ApiKey(&account.key),
        )
        .await;
    assert_status(&mainnet_invoices, StatusCode::OK);
    assert!(mainnet_invoices.parse::<Vec<Invoice>>().is_empty());

    let incompatible_invoice = app
        .api()
        .post(
            &format!("/v1/me/wallets/{}/invoices", mainnet.id),
            Auth::ApiKey(&account.key),
            NewInvoiceRequest {
                wallet_id: None,
                amount_msat: 25_000,
                description: None,
                expiry: None,
            },
        )
        .await;
    assert_error(&incompatible_invoice, StatusCode::UNPROCESSABLE_ENTITY);

    let incompatible_payment = app
        .api()
        .post(
            &format!("/v1/me/wallets/{}/payments", mainnet.id),
            Auth::ApiKey(&account.key),
            SendPaymentRequest {
                wallet_id: None,
                input: invoice.ln_invoice.expect("Lightning invoice").bolt11,
                amount_msat: None,
                comment: None,
            },
        )
        .await;
    assert_error(&incompatible_payment, StatusCode::UNPROCESSABLE_ENTITY);
    let unchanged_mainnet_balance = balance(app, &account.key, mainnet.id).await;
    assert_eq!(unchanged_mainnet_balance.available_msat, 0);
    assert_eq!(unchanged_mainnet_balance.reserved_msat, 0);

    let overviews = app.api().get("/v1/wallets/overviews", Auth::Bearer(admin)).await;
    assert_status(&overviews, StatusCode::OK);
    let overviews = overviews.parse::<Vec<WalletOverview>>();
    let regtest_overview = overview(&overviews, regtest.id);
    let mainnet_overview = overview(&overviews, mainnet.id);
    let signet_overview = overview(&overviews, signet.id);
    assert_eq!(regtest_overview.balance.available_msat, regtest_balance.available_msat);
    assert!(
        regtest_overview.n_invoices >= 1,
        "the regtest overview includes its Lightning and deposit activity"
    );
    assert_eq!(mainnet_overview.balance.available_msat, 0);
    assert_eq!(mainnet_overview.n_invoices, 0);
    assert_eq!(signet_overview.balance.available_msat, 0);
    assert_eq!(signet_overview.n_invoices, 0);
}

async fn create_wallet(app: &crate::common::TestApp, key: &str, asset_id: uuid::Uuid) -> Wallet {
    let response = app
        .api()
        .post(
            "/v1/me/wallets",
            Auth::ApiKey(key),
            CreateWalletRequest {
                account_id: None,
                asset_id,
            },
        )
        .await;
    assert_status(&response, StatusCode::OK);
    response.parse::<Wallet>()
}

async fn balance(app: &crate::common::TestApp, key: &str, wallet_id: uuid::Uuid) -> Balance {
    let response = app
        .api()
        .get(&format!("/v1/me/wallets/{wallet_id}/balance"), Auth::ApiKey(key))
        .await;
    assert_status(&response, StatusCode::OK);
    response.parse::<Balance>()
}

fn assert_network(wallets: &[Wallet], wallet_id: uuid::Uuid, expected: BtcNetwork) {
    let wallet = wallets
        .iter()
        .find(|wallet| wallet.id == wallet_id)
        .expect("wallet is listed");
    assert_eq!(wallet.asset.as_ref().expect("asset metadata").network, expected);
}

fn overview(overviews: &[WalletOverview], wallet_id: uuid::Uuid) -> &WalletOverview {
    overviews
        .iter()
        .find(|overview| overview.id == wallet_id)
        .expect("wallet overview exists")
}

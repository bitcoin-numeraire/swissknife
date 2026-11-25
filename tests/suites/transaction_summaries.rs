//! Administrative wallet labels follow the reader's existing account and wallet scopes.
use reqwest::StatusCode;
use sea_orm::{ConnectionTrait, Database, Statement};
use serde_json::json;
use uuid::Uuid;

use crate::common::{app, assert_status, Auth};
use swissknife_types::Permission;

#[tokio::test]
async fn administrative_transaction_summaries_limit_ownership_fields_to_authorized_readers() {
    let app = app().await;
    let admin = app.admin_token().await;
    let wallet = app.create_wallet(admin, "Treasury desk").await;
    let owner = app.account_api_key(admin, wallet.account_id, vec![]).await;
    let read_tx = app.api_key(admin, vec![Permission::ReadTransaction]).await;
    let read_wallet = app
        .api_key(admin, vec![Permission::ReadTransaction, Permission::ReadWallet])
        .await;
    let read_account = app
        .api_key(admin, vec![Permission::ReadTransaction, Permission::ReadAccount])
        .await;
    let db = Database::connect(&app.database_url).await.unwrap();
    let backend = db.get_database_backend();
    db.execute_raw(Statement::from_sql_and_values(
        backend,
        "UPDATE wallet SET label = 'Cash desk' WHERE id = $1",
        [wallet.id.into()],
    ))
    .await
    .unwrap();
    db.execute_raw(Statement::from_sql_and_values(
        backend,
        "INSERT INTO payment (id, wallet_id, amount_msat, status, ledger, reserved_amount, created_at)
        VALUES ($1, $2, 1000, 'Failed', 'Internal', 0, CURRENT_TIMESTAMP)",
        [Uuid::new_v4().into(), wallet.id.into()],
    ))
    .await
    .unwrap();
    let invoice = app
        .api()
        .post(
            &format!("/v1/me/wallets/{}/invoices", wallet.id),
            Auth::ApiKey(&owner),
            json!({"amount_msat": 1000}),
        )
        .await;
    assert_status(&invoice, StatusCode::OK);

    for kind in ["invoices", "payments"] {
        let route = format!("/v1/{kind}?wallet_id={}", wallet.id);
        for (key, has_wallet, has_account) in [
            (&read_tx, false, false),
            (&read_wallet, true, false),
            (&read_account, true, true),
        ] {
            let res = app.api().get(&route, Auth::ApiKey(key)).await;
            assert_status(&res, StatusCode::OK);
            let rows = res.body.as_array().unwrap();
            assert_eq!(rows.len(), 1);
            let display = rows[0].get("wallet_display");
            assert_eq!(display.is_some(), has_wallet);
            if let Some(display) = display {
                assert_eq!(display["wallet_label"], "Cash desk");
                assert_eq!(display["display_ticker"], wallet.asset.as_ref().unwrap().display_ticker);
                assert_eq!(display["network"], "Regtest");
                assert_eq!(display.get("account_id").is_some(), has_account);
                assert_eq!(display.get("account_label").is_some(), has_account);
                if has_account {
                    assert_eq!(display["account_label"], "Treasury desk");
                }
            }
        }
        let route = format!("/v1/me/wallets/{}/{kind}", wallet.id);
        let res = app.api().get(&route, Auth::ApiKey(&owner)).await;
        assert_status(&res, StatusCode::OK);
        assert!(res.body[0].get("wallet_display").is_none());
        assert_status(
            &app.api().get(&route, Auth::ApiKey(&read_account)).await,
            StatusCode::NOT_FOUND,
        );
    }
}

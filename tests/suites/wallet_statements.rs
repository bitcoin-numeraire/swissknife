//! Exercise the downloadable CSV through both authorization paths on the real database.
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use sea_orm::{ConnectionTrait, Database, Statement, TransactionTrait};
use swissknife_types::Permission;
use uuid::Uuid;

use crate::common::{assert_status, Auth, TestApp};

#[tokio::test]
async fn wallet_statement_preserves_units_boundaries_order_and_ownership() {
    let app = TestApp::isolated("wallet-statement", &[]).await;
    let admin = app.admin_token().await;
    let wallet = app.create_wallet(admin, "statement-owner").await;
    let owner = app.account_api_key(admin, wallet.account_id, vec![]).await;
    let reader = app.api_key(admin, vec![Permission::ReadTransaction]).await;
    let other = app.create_wallet(admin, "other-statement-owner").await;
    let from: DateTime<Utc> = "2025-03-30T00:00:00Z".parse().unwrap();
    let until: DateTime<Utc> = "2025-03-31T00:00:00Z".parse().unwrap();
    let db = Database::connect(&app.database_url).await.unwrap();
    let txn = db.begin().await.unwrap();
    let backend = txn.get_database_backend();
    let amount = 9_007_199_254_740_993_i64;
    let memo = " +tax, \"tip\"\nMarch";
    let mut expected = Vec::new();
    for index in 0..104 {
        let id = Uuid::new_v4();
        let target = if index == 103 { other.id } else { wallet.id };
        let time = match index {
            101 => until,
            102 => from - chrono::Duration::microseconds(1),
            _ => from,
        };
        txn.execute_raw(Statement::from_sql_and_values(
            backend,
            "INSERT INTO payment
            (id, wallet_id, amount_msat, fee_msat, status, ledger, description, created_at, reserved_amount)
            VALUES ($1, $2, $3, 17, 'Failed', 'Lightning', $4, $5, 0)",
            [
                id.into(),
                target.into(),
                amount.into(),
                memo.into(),
                time.naive_utc().into(),
            ],
        ))
        .await
        .unwrap();
        if index < 101 {
            expected.push(id.to_string());
        }
    }
    let invoice_id = Uuid::new_v4();
    txn.execute_raw(Statement::from_sql_and_values(
        backend,
        "INSERT INTO invoice
        (id,wallet_id,ledger,description,amount_msat,amount_received_msat,timestamp,created_at,payment_time)
        VALUES ($1,$2,'Internal','received',1000,1200,$3,$3,$3)",
        [invoice_id.into(), wallet.id.into(), from.naive_utc().into()],
    ))
    .await
    .unwrap();
    txn.commit().await.unwrap();
    expected.push(invoice_id.to_string());
    expected.sort();

    let query = "from=2025-03-30T00:00:00Z&until=2025-03-31T00:00:00Z";
    let self_route = format!("/v1/me/wallets/{}/statement?{query}", wallet.id);
    let admin_route = format!("/v1/wallets/{}/statement?{query}", wallet.id);
    let self_res = reqwest::Client::new()
        .get(format!("{}{}", app.base_url, self_route))
        .header("api-key", &owner)
        .send()
        .await
        .unwrap();
    assert_eq!(self_res.status(), StatusCode::OK);
    assert!(self_res.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("text/csv"));
    assert!(self_res.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .contains(&wallet.id.to_string()));
    assert!(self_res.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let body = self_res.text().await.unwrap();
    let admin_res = app.api().get(&admin_route, Auth::ApiKey(&reader)).await;
    assert_status(&admin_res, StatusCode::OK);
    assert_eq!(admin_res.body.as_str(), Some(body.as_str()));
    let mut csv = csv::Reader::from_reader(body.as_bytes());
    assert_eq!(csv.headers().unwrap().get(9), Some("gross_amount_msat"));
    let rows = csv.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(rows.iter().map(|r| r[0].to_owned()).collect::<Vec<_>>(), expected);
    for row in rows {
        assert_eq!(row[1], wallet.id.to_string());
        assert_eq!(row[2], wallet.asset_id.to_string());
        assert_eq!(&row[3], "Regtest");
        assert_eq!(&row[4], "2025-03-30T00:00:00Z");
        if row[0] == invoice_id.to_string() {
            assert_eq!(
                (&row[6], &row[7], &row[8], &row[9]),
                ("in", "Internal", "Settled", "1200")
            );
            assert_eq!(&row[5], "2025-03-30T00:00:00Z");
        } else {
            assert_eq!(
                (&row[6], &row[8], &row[9], &row[10]),
                ("out", "Failed", "9007199254740993", "17")
            );
            assert_eq!(row[11], format!("'{memo}"));
        }
    }
    for (route, auth, status) in [
        (admin_route.clone(), Auth::ApiKey(&owner), StatusCode::FORBIDDEN),
        (self_route.clone(), Auth::ApiKey(&reader), StatusCode::NOT_FOUND),
        (self_route.clone(), Auth::None, StatusCode::UNAUTHORIZED),
        (
            format!("/v1/wallets/{}/statement?{query}", Uuid::new_v4()),
            Auth::ApiKey(&reader),
            StatusCode::NOT_FOUND,
        ),
        (
            self_route.replace("2025-03-31", "2025-03-29"),
            Auth::ApiKey(&owner),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_status(&app.api().get(&route, auth).await, status);
    }
    let empty = app
        .api()
        .get(&self_route.replace("2025-03", "2024-03"), Auth::ApiKey(&owner))
        .await;
    assert_status(&empty, StatusCode::OK);
    assert_eq!(empty.body.as_str().unwrap().lines().count(), 1);
}

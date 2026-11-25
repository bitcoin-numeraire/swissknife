//! Pay independently issued offers and verify the negotiated invoice on its issuer.
use crate::common::{
    assert_error, assert_status, counterparty::Counterparty, fixtures::unique, wait::wait_until, Auth, TestApp,
};
use reqwest::StatusCode;
use serde_json::json;
use std::time::Duration;
use swissknife_types::{Payment, PaymentExecutionState, PaymentSubmission};
use uuid::Uuid;

#[tokio::test]
async fn sends_fixed_and_amountless_bolt12_requests_and_enforces_the_payer_amount() {
    let app = TestApp::isolated("bolt12-send", &[]).await;
    let admin = app.admin_token().await;
    let account = app.create_account_with_wallet(admin, "offers-payer").await;
    let api = app.api();
    let auth = Auth::ApiKey(&account.key);
    let path = format!("/v1/me/wallets/{}/payments", account.wallet.id);
    if app.provider.starts_with("lnd") {
        let result = api
            .post(
                &format!("{path}/submissions"),
                auth,
                json!({"input":"lno1unsupported","idempotency_key":Uuid::new_v4(),"amount_msat":10000}),
            )
            .await;
        assert_error(&result, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(result.body["reason"]
            .as_str()
            .unwrap()
            .contains("does not support BOLT12"));
        return;
    }
    app.fund_onchain(admin, account.wallet.id, 100_000).await;
    let peer = Counterparty::offers_peer();
    let fixed = peer.cln_rpc(
        "offer",
        json!({"amount":"42000msat","description":unique("fixed offer")}),
    )["bolt12"]
        .as_str()
        .unwrap()
        .to_owned();
    let variable = peer.cln_rpc("offer", json!({"amount":"any","description":unique("flexible offer")}))["bolt12"]
        .as_str()
        .unwrap()
        .to_owned();
    let status_path = format!("{path}/submissions");
    let mismatch = api
        .post(
            &status_path,
            auth,
            json!({"input":fixed,"amount_msat":43000,"idempotency_key":Uuid::new_v4()}),
        )
        .await;
    assert_error(&mismatch, StatusCode::UNPROCESSABLE_ENTITY);
    let missing = api
        .post(
            &status_path,
            auth,
            json!({"input":variable,"idempotency_key":Uuid::new_v4()}),
        )
        .await;
    assert_error(&missing, StatusCode::UNPROCESSABLE_ENTITY);
    let quote = api
        .post(&format!("{path}/fee-estimate"), auth, json!({"input":fixed}))
        .await;
    assert_status(&quote, StatusCode::OK);
    assert_eq!(quote.parse::<serde_json::Value>()["amount_msat"], 42000);
    let body = json!({"input":fixed,"comment":"one approved order","idempotency_key":Uuid::new_v4()});
    let first = api.post(&status_path, auth, &body).await;
    assert_status(&first, StatusCode::ACCEPTED);
    let first: PaymentSubmission = first.parse();
    let replay: PaymentSubmission = api.post(&status_path, auth, &body).await.parse();
    assert_eq!(replay.payment.id, first.payment.id);
    wait_until(Duration::from_secs(35), "offer settles", || async {
        api.get(&first.status_url, auth)
            .await
            .parse::<PaymentSubmission>()
            .state
            == PaymentExecutionState::Settled
    })
    .await;
    let hash = first.payment.lightning.unwrap().payment_hash;
    let issued = peer.cln_rpc("listinvoices", json!({"payment_hash":hash}));
    assert_eq!(issued["invoices"][0]["status"], "paid");
    assert_eq!(issued["invoices"][0]["invreq_payer_note"], "one approved order");
    let paid = api
        .post(&path, auth, json!({"input":variable,"amount_msat":47000}))
        .await;
    assert_status(&paid, StatusCode::OK);
    assert_eq!(paid.parse::<Payment>().amount_msat, 47000);
    let source = Counterparty::for_provider("lnd_grpc");
    let negotiated = source.cln_rpc("fetchinvoice", json!({"offer":fixed,"timeout":15}));
    let paid = api.post(&path, auth, json!({"input":negotiated["invoice"]})).await;
    assert_status(&paid, StatusCode::OK);
    assert_eq!(paid.parse::<Payment>().amount_msat, 42000);
    assert_eq!(app.wallet_balance(admin, account.wallet.id).await.reserved_msat, 0);
}

#[tokio::test]
async fn receives_reusable_offers_once_per_paid_invoice_after_restart() {
    use swissknife_types::{Invoice, OfferState, WalletOffer};
    let app = TestApp::isolated("bolt12-receive", &[]).await;
    let admin = app.admin_token().await.to_owned();
    let a = app.create_account_with_wallet(&admin, "offer-shop").await;
    let b = app.create_account_with_wallet(&admin, "offer-other").await;
    let path = format!("/v1/me/wallets/{}/offers", a.wallet.id);
    let other = format!("/v1/me/wallets/{}/offers", b.wallet.id);
    let auth = Auth::ApiKey(&a.key);
    let api = app.api();
    let body = json!({"idempotency_key":Uuid::new_v4(),"description":"Two shops, same label"});
    assert_error(&api.get(&path, Auth::None).await, StatusCode::UNAUTHORIZED);
    assert_error(&api.get(&path, Auth::ApiKey(&b.key)).await, StatusCode::NOT_FOUND);
    let result = api.post(&path, auth, &body).await;
    if app.provider.starts_with("lnd") {
        assert_error(&result, StatusCode::UNPROCESSABLE_ENTITY);
        return;
    }
    assert_status(&result, StatusCode::OK);
    let first: WalletOffer = result.parse();
    assert_eq!(first.state, OfferState::Active);
    let wallet_path = format!("/v1/wallets/{}", a.wallet.id);
    let account_path = format!("/v1/accounts/{}", a.account.id);
    for path in [&wallet_path, &account_path] {
        assert_error(&api.delete(path, Auth::Bearer(&admin)).await, StatusCode::CONFLICT);
    }
    let clean = app.create_account_with_wallet(&admin, "offer-bulk-delete").await;
    let wallet_batch = format!("/v1/wallets?ids={}&ids={}", clean.wallet.id, a.wallet.id);
    let account_batch = format!("/v1/accounts?ids={}&ids={}", clean.account.id, a.account.id);
    for batch in [wallet_batch, account_batch] {
        assert_error(&api.delete(&batch, Auth::Bearer(&admin)).await, StatusCode::CONFLICT);
    }
    assert_status(
        &api.get(&format!("/v1/wallets/{}", clean.wallet.id), Auth::Bearer(&admin))
            .await,
        StatusCode::OK,
    );
    let scoped = format!("{path}/{}", first.id);
    let op = format!("/v1/offers/{}", first.id);
    assert_eq!(api.get(&scoped, auth).await.parse::<WalletOffer>().id, first.id);
    assert_eq!(
        api.get(&op, Auth::Bearer(&admin))
            .await
            .parse::<WalletOffer>()
            .wallet_id,
        a.wallet.id
    );
    assert_error(&api.get(&op, Auth::None).await, StatusCode::UNAUTHORIZED);
    use swissknife_types::Permission;
    let service = app.create_account(&admin, "publisher").await;
    let reader = app
        .account_api_key(&admin, service.id, vec![Permission::ReadTransaction])
        .await;
    let limited = app
        .account_api_key(&admin, service.id, vec![Permission::ReadWallet])
        .await;
    assert_eq!(
        api.get(&op, Auth::ApiKey(&reader)).await.parse::<WalletOffer>().id,
        first.id
    );
    assert_error(&api.delete(&op, Auth::ApiKey(&reader)).await, StatusCode::FORBIDDEN);
    assert_error(&api.get(&op, Auth::ApiKey(&limited)).await, StatusCode::FORBIDDEN);
    assert_error(&api.get(&scoped, Auth::ApiKey(&b.key)).await, StatusCode::NOT_FOUND);
    assert_error(
        &api.get(&format!("{other}/{}", first.id), Auth::ApiKey(&b.key)).await,
        StatusCode::NOT_FOUND,
    );
    let replay: WalletOffer = api.post(&path, auth, &body).await.parse();
    assert_eq!(replay.bolt12, first.bolt12);
    let mut conflict = body.clone();
    conflict["amount_msat"] = json!(1000);
    assert_error(&api.post(&path, auth, conflict).await, StatusCode::CONFLICT);
    let mut second = body;
    second["single_use"] = json!(true);
    let second: WalletOffer = api.post(&other, Auth::ApiKey(&b.key), second).await.parse();
    assert_ne!(first.provider_id, second.provider_id);
    let admin_list = api
        .get(&format!("/v1/offers?wallet_id={}", a.wallet.id), Auth::Bearer(&admin))
        .await;
    assert_eq!(admin_list.parse::<Vec<WalletOffer>>().len(), 1);
    assert_error(
        &api.delete(&format!("{other}/{}", first.id), Auth::ApiKey(&b.key)).await,
        StatusCode::NOT_FOUND,
    );
    let peer = Counterparty::offers_peer();
    let pay = |offer: &str, amount: u64, note: &str| {
        let invoice = peer.cln_rpc(
            "fetchinvoice",
            json!({"offer":offer,"amount_msat":amount,"payer_note":note,"timeout":15}),
        );
        peer.cln_rpc("xpay", json!({"invstring":invoice["invoice"]}));
    };
    let remote = Counterparty::for_provider("lnd_grpc");
    let foreign = remote.cln_rpc("offer", json!({"amount":"any","description":unique("operator offer")}));
    app.stop();
    pay(first.bolt12.as_ref().unwrap(), 12017, "order A");
    pay(first.bolt12.as_ref().unwrap(), 19231, "order B");
    pay(second.bolt12.as_ref().unwrap(), 33107, "single");
    pay(foreign["bolt12"].as_str().unwrap(), 1013, "unregistered");
    let app = app.restart("offer-replay", &[]).await;
    let api = app.api();
    wait_until(Duration::from_secs(35), "catch up paid BOLT12 offers", || async {
        app.wallet_balance(&admin, a.wallet.id).await.available_msat == 31248
            && app.wallet_balance(&admin, b.wallet.id).await.available_msat == 33107
    })
    .await;
    let history = format!("/v1/me/wallets/{}/invoices", a.wallet.id);
    let invoices: Vec<Invoice> = api.get(&history, auth).await.parse();
    assert_eq!(invoices.len(), 2);
    let mut seen: Vec<_> = invoices
        .iter()
        .map(|inv| {
            assert!(inv.ln_invoice.is_none());
            let paid = inv.offer_invoice.as_ref().unwrap();
            assert_eq!(paid.offer_id, first.id);
            assert!(paid.bolt12.starts_with("lni1"));
            (inv.amount_received_msat.unwrap(), paid.payer_note.clone().unwrap())
        })
        .collect();
    seen.sort();
    assert_eq!(seen, vec![(12017, "order A".into()), (19231, "order B".into())]);
    let status: Vec<WalletOffer> = api.get(&other, Auth::ApiKey(&b.key)).await.parse();
    assert_eq!(status[0].state, OfferState::Disabled);
    let wallet_payments = format!("/v1/me/wallets/{}/payments", a.wallet.id);
    assert_error(
        &api.post(&wallet_payments, auth, json!({"input":first.bolt12,"amount_msat":1005}))
            .await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    let cross = api
        .post(
            &format!("/v1/me/wallets/{}/payments", b.wallet.id),
            Auth::ApiKey(&b.key),
            json!({"input":first.bolt12,"amount_msat":1005}),
        )
        .await;
    assert_status(&cross, StatusCode::OK);
    let fee = cross.parse::<Payment>().fee_msat.unwrap();
    wait_until(Duration::from_secs(15), "different wallet on the same node", || async {
        app.wallet_balance(&admin, a.wallet.id).await.available_msat == 32253
            && app.wallet_balance(&admin, b.wallet.id).await.available_msat == 33107 - 1005 - fee as i64
    })
    .await;
    let disable = format!("{path}/{}", first.id);
    for _ in 0..2 {
        assert_eq!(
            api.delete(&disable, auth).await.parse::<WalletOffer>().state,
            OfferState::Disabled
        );
    }
    let registered = remote.cln_rpc("listoffers", json!({"offer_id":first.provider_id}));
    assert_eq!(registered["offers"][0]["active"], false);
    let restarted = app.restart("offers-up-to-date", &[]).await;
    assert_eq!(
        restarted.wallet_balance(&admin, a.wallet.id).await.available_msat,
        32253
    );
    // The node is now inactive and all receipts have been recorded, including the missed ones.
    wait_until(Duration::from_secs(30), "offer ownership released", || async {
        restarted
            .api()
            .delete(&account_path, Auth::Bearer(&admin))
            .await
            .status
            .is_success()
    })
    .await;
}

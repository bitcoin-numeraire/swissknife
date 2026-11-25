use crate::common::{
    assert_error, assert_status, chain::new_address, counterparty::Counterparty, fixtures::unique, wait::wait_until,
    Auth, MockLnurl, TestApp,
};
use chrono::{Duration as TimeDelta, Utc};
use reqwest::StatusCode;
use serde_json::json;
use std::time::Duration;
use swissknife_types::{IssuedAgentKey, PaymentExecutionState, PaymentSubmission};
use uuid::Uuid;

#[tokio::test]
async fn an_operator_can_approve_one_credential_retry_of_the_original_invoice() {
    let app = TestApp::isolated("approval-lnurl", &[]).await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "approval-owner").await;
    let other = app.create_account_with_wallet(admin, "approval-other").await;
    app.fund_onchain(admin, owner.wallet.id, 75_000).await;
    let api = app.api();
    let wallet = format!("/v1/me/wallets/{}", owner.wallet.id);
    let keys = format!("{wallet}/agent-keys");
    let key_request = json!({"name":"bounded integration","capabilities":{"spend":true,"read":true},"expires_at":Utc::now()+TimeDelta::hours(1)});
    let agent = api
        .post(&keys, Auth::ApiKey(&owner.key), &key_request)
        .await
        .parse::<IssuedAgentKey>();
    let sibling = api
        .post(&keys, Auth::ApiKey(&owner.key), &key_request)
        .await
        .parse::<IssuedAgentKey>();
    let actor = Auth::WalletKey(&agent.key);
    let account = format!("/v1/accounts/{}", owner.account.id);
    let policy = format!("{account}/spending-policy");
    let queue = format!("{account}/payment-approvals");
    let submit = format!("{wallet}/payments/submissions");
    let operator = Auth::Bearer(admin);
    let limits = |daily: &str| json!({"approval_above_msat":"35000","daily_msat":daily});
    assert_status(
        &api.put(&policy, operator, limits("100000")).await,
        StatusCode::NO_CONTENT,
    );
    let restricted = app.account_api_key(admin, owner.account.id, vec![]).await;
    assert_error(&api.get(&queue, Auth::ApiKey(&restricted)).await, StatusCode::FORBIDDEN);
    assert_error(&api.get(&queue, actor).await, StatusCode::UNAUTHORIZED);
    let preimage = rand::random::<[u8; 32]>();
    let invoice =
        Counterparty::for_provider(&app.provider).invoice_with_preimage(35_000, &unique("reviewed"), &preimage);
    let mock = MockLnurl::start().await;
    let payee = unique("reviewed-payee");
    mock.mount_pay_request(&payee, 1_000, 1_000_000, 255).await;
    mock.mount_callback_invoice_with_aes(&invoice, "Receipt", "Original invoice paid", &preimage)
        .await;
    let input = mock.lnurlp_url(&payee);
    let request = json!({"idempotency_key":Uuid::new_v4(),"input":input,"amount_msat":35000,"maximum_fee_msat":1000,"comment":"order"});
    let first = api.post(&submit, actor, &request).await;
    assert_error(&first, StatusCode::CONFLICT);
    assert_eq!(first.body["code"], "payment_approval_required");
    drop(mock);
    let pending = api.get(&queue, operator).await.body;
    assert_eq!(pending.as_array().unwrap().len(), 1);
    let item = &pending[0];
    assert_eq!(item["amount_msat"], "35000");
    assert_eq!(item["maximum_fee_msat"], "1000");
    assert_eq!(item["requested_input"], input);
    assert_eq!(item["agent_key_id"], agent.metadata.id.to_string());
    let hash = invoice
        .parse::<lightning_invoice::Bolt11Invoice>()
        .unwrap()
        .payment_hash()
        .to_string();
    assert_eq!(item["destination"], hash);
    let id = item["id"].as_str().unwrap();
    let decision = format!("{queue}/{id}");
    let wrong = format!("/v1/accounts/{}/payment-approvals/{id}", other.account.id);
    let yes = json!({"decision":"approve"});
    assert_error(&api.put(&wrong, operator, &yes).await, StatusCode::NOT_FOUND);
    assert_error(&api.put(&decision, actor, &yes).await, StatusCode::UNAUTHORIZED);
    assert_error(
        &api.put(&decision, Auth::ApiKey(&restricted), &yes).await,
        StatusCode::FORBIDDEN,
    );
    assert_error(&api.post(&submit, actor, &request).await, StatusCode::CONFLICT);
    assert_eq!(api.get(&queue, operator).await.body.as_array().unwrap().len(), 1);
    let before = app.wallet_balance(admin, owner.wallet.id).await;
    assert_eq!(before.reserved_msat, 0);
    assert_eq!(api.get(&policy, operator).await.body["usage"]["daily_msat"], "0");
    assert_eq!(api.put(&decision, operator, &yes).await.body["state"], "approved");
    assert_eq!(api.put(&decision, operator, &yes).await.body["state"], "approved");
    assert_error(
        &api.put(&decision, operator, json!({"decision":"reject"})).await,
        StatusCode::CONFLICT,
    );
    assert_eq!(
        app.wallet_balance(admin, owner.wallet.id).await.available_msat,
        before.available_msat
    );
    assert_error(
        &api.post(&submit, Auth::WalletKey(&sibling.key), &request).await,
        StatusCode::CONFLICT,
    );
    let mut altered = request.clone();
    altered["maximum_fee_msat"] = json!(1001);
    assert_error(&api.post(&submit, actor, altered).await, StatusCode::CONFLICT);
    assert_status(
        &api.put(&policy, operator, limits("1000")).await,
        StatusCode::NO_CONTENT,
    );
    assert_error(
        &api.post(&submit, actor, &request).await,
        StatusCode::UNPROCESSABLE_ENTITY,
    );
    assert_eq!(api.get(&decision, operator).await.body["state"], "approved");
    assert_status(
        &api.put(&policy, operator, limits("100000")).await,
        StatusCode::NO_CONTENT,
    );
    let app = app.restart("approval-retry", &[]).await;
    let api = app.api();
    let (a, b) = tokio::join!(api.post(&submit, actor, &request), api.post(&submit, actor, &request));
    assert_status(&a, StatusCode::ACCEPTED);
    assert_status(&b, StatusCode::ACCEPTED);
    let a = a.parse::<PaymentSubmission>();
    assert_eq!(a.payment.id, b.parse::<PaymentSubmission>().payment.id);
    wait_until(
        Duration::from_secs(40),
        "the originally reviewed invoice settles once with its original receipt",
        || async {
            let state = api.get(&a.status_url, actor).await.parse::<PaymentSubmission>();
            state.state == PaymentExecutionState::Settled
                && state
                    .payment
                    .lightning
                    .and_then(|ln| ln.success_action)
                    .and_then(|a| a.message)
                    .as_deref()
                    == Some("Original invoice paid")
        },
    )
    .await;
    let operator = Auth::Bearer(app.admin_token().await);
    assert_eq!(api.get(&decision, operator).await.body["state"], "consumed");
    assert_eq!(api.get(&queue, operator).await.body, json!([]));
    assert_eq!(
        api.post(&submit, actor, &request)
            .await
            .parse::<PaymentSubmission>()
            .payment
            .id,
        a.payment.id
    );
    assert_eq!(api.get(&policy, operator).await.body["usage"]["daily_msat"], "35000");
    app.stop();
}

#[tokio::test]
async fn rejected_or_revoked_delegations_cannot_admit_withdrawals() {
    let app = TestApp::isolated("approval-withdrawal", &[]).await;
    let admin = app.admin_token().await;
    let operator = Auth::Bearer(admin);
    let owner = app.create_account_with_wallet(admin, "approval-withdrawal").await;
    app.fund_onchain(admin, owner.wallet.id, 100_000).await;
    let api = app.api();
    let wallet = format!("/v1/me/wallets/{}", owner.wallet.id);
    let keys = format!("{wallet}/agent-keys");
    let agent = api
        .post(
            &keys,
            Auth::ApiKey(&owner.key),
            json!({"name":"withdraw","capabilities":{"spend":true},"expires_at":Utc::now()+TimeDelta::hours(1)}),
        )
        .await
        .parse::<IssuedAgentKey>();
    let actor = Auth::WalletKey(&agent.key);
    let account = format!("/v1/accounts/{}", owner.account.id);
    let policy = format!("{account}/spending-policy");
    let queue = format!("{account}/payment-approvals");
    let pay = format!("{wallet}/payments/submissions");
    assert_status(
        &api.put(&policy, operator, json!({"approval_above_msat":"0"})).await,
        StatusCode::NO_CONTENT,
    );
    let address = new_address().await;
    let mut request =
        json!({"input":address,"amount_msat":1500000,"maximum_fee_msat":1000000,"idempotency_key":Uuid::new_v4()});
    let start = app.wallet_balance(admin, owner.wallet.id).await;
    for verdict in ["reject", "approve"] {
        request["idempotency_key"] = json!(Uuid::new_v4());
        assert_error(&api.post(&pay, actor, &request).await, StatusCode::CONFLICT);
        let items = api.get(&queue, operator).await.body;
        let item = &items[0];
        assert_eq!(item["destination"], address);
        assert_eq!(item["amount_msat"], "1500000");
        let path = format!("{queue}/{}", item["id"].as_str().unwrap());
        assert_status(
            &api.put(&path, operator, json!({"decision":verdict})).await,
            StatusCode::OK,
        );
        if verdict == "reject" {
            assert_error(&api.post(&pay, actor, &request).await, StatusCode::CONFLICT);
        } else {
            assert_status(
                &api.delete(&format!("{keys}/{}", agent.metadata.id), Auth::ApiKey(&owner.key))
                    .await,
                StatusCode::NO_CONTENT,
            );
            assert_error(&api.post(&pay, actor, &request).await, StatusCode::UNAUTHORIZED);
        }
    }
    let after = app.wallet_balance(admin, owner.wallet.id).await;
    assert_eq!(start.available_msat, after.available_msat);
    assert_eq!(after.reserved_msat, 0);
    assert_eq!(api.get(&policy, operator).await.body["usage"]["daily_msat"], "0");
    app.stop();
}

#[tokio::test]
async fn exact_threshold_and_owner_transfers_proceed_while_approved_internal_transfers_debit_once() {
    let app = TestApp::isolated("approval-internal", &[]).await;
    let admin = app.admin_token().await;
    let operator = Auth::Bearer(admin);
    let owner = app.create_account_with_wallet(admin, "review-source").await;
    let recipient = app.create_account_with_wallet(admin, "review-recipient").await;
    app.fund_onchain(admin, owner.wallet.id, 20_000).await;
    let api = app.api();
    let wallet = format!("/v1/me/wallets/{}", owner.wallet.id);
    let key = api
        .post(
            &format!("{wallet}/agent-keys"),
            Auth::ApiKey(&owner.key),
            json!({"name":"inside","capabilities":{"spend":true},"expires_at":Utc::now()+TimeDelta::hours(1)}),
        )
        .await
        .parse::<IssuedAgentKey>();
    let actor = Auth::WalletKey(&key.key);
    let account = format!("/v1/accounts/{}", owner.account.id);
    let policy = format!("{account}/spending-policy");
    let queue = format!("{account}/payment-approvals");
    let pay = format!("{wallet}/payments/submissions");
    let address = api
        .post(
            "/v1/bitcoin/addresses",
            operator,
            json!({"wallet_id":recipient.wallet.id}),
        )
        .await
        .body["address"]
        .as_str()
        .unwrap()
        .to_string();
    let send = |amount: u64| json!({"input":address,"amount_msat":amount,"maximum_fee_msat":0,"idempotency_key":Uuid::new_v4()});
    assert_status(
        &api.put(&policy, operator, json!({"approval_above_msat":"5000"})).await,
        StatusCode::NO_CONTENT,
    );
    assert_status(&api.post(&pay, actor, send(5000)).await, StatusCode::ACCEPTED);
    assert_status(
        &api.post(&pay, Auth::ApiKey(&owner.key), send(6000)).await,
        StatusCode::ACCEPTED,
    );
    assert_eq!(api.get(&queue, operator).await.body, json!([]));
    let req = send(7000);
    let (one, two) = tokio::join!(api.post(&pay, actor, &req), api.post(&pay, actor, &req));
    assert_eq!(one.body["code"], "payment_approval_required");
    assert_eq!(two.body["code"], one.body["code"]);
    let items = api.get(&queue, operator).await.body;
    assert_eq!(items.as_array().unwrap().len(), 1);
    let row = &items[0];
    assert_eq!(row["ledger"], "Internal");
    assert_eq!(row["destination"], address);
    assert_eq!(row["maximum_fee_msat"], "0");
    assert_eq!(
        app.wallet_balance(admin, recipient.wallet.id).await.available_msat,
        11000
    );
    let review = format!("{queue}/{}", row["id"].as_str().unwrap());
    let (yes, no) = tokio::join!(
        api.put(&review, operator, json!({"decision":"approve"})),
        api.put(&review, operator, json!({"decision":"reject"}))
    );
    let mut statuses = [yes.status.as_u16(), no.status.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    if yes.status == StatusCode::OK {
        let (first, second) = tokio::join!(api.post(&pay, actor, &req), api.post(&pay, actor, &req));
        assert_status(&first, StatusCode::ACCEPTED);
        assert_status(&second, StatusCode::ACCEPTED);
        assert_eq!(first.body["payment"]["id"], second.body["payment"]["id"]);
        assert_eq!(
            app.wallet_balance(admin, recipient.wallet.id).await.available_msat,
            18000
        );
    } else {
        assert_error(&api.post(&pay, actor, &req).await, StatusCode::CONFLICT);
        assert_eq!(
            app.wallet_balance(admin, recipient.wallet.id).await.available_msat,
            11000
        );
    }
    app.stop();
}

#[tokio::test]
async fn a_cln_offer_approval_settles_the_invoice_hash_shown_to_the_operator() {
    let app = TestApp::isolated("approval-offer", &[]).await;
    if app.provider.starts_with("lnd") {
        app.stop();
        return;
    }
    let admin = app.admin_token().await;
    let operator = Auth::Bearer(admin);
    let owner = app.create_account_with_wallet(admin, "offer-review").await;
    app.fund_onchain(admin, owner.wallet.id, 50_000).await;
    let api = app.api();
    let wallet = format!("/v1/me/wallets/{}", owner.wallet.id);
    let key=api.post(&format!("{wallet}/agent-keys"),Auth::ApiKey(&owner.key),json!({"name":"buyer","capabilities":{"spend":true,"read":true},"expires_at":Utc::now()+TimeDelta::hours(1)})).await.parse::<IssuedAgentKey>();
    let actor = Auth::WalletKey(&key.key);
    let account = format!("/v1/accounts/{}", owner.account.id);
    assert_status(
        &api.put(
            &format!("{account}/spending-policy"),
            operator,
            json!({"approval_above_msat":"1000"}),
        )
        .await,
        StatusCode::NO_CONTENT,
    );
    let peer = Counterparty::offers_peer();
    let label = unique("approval-offer");
    let offer = peer.cln_rpc("offer", json!({"amount":"32000msat","description":label}));
    let request = json!({"input":offer["bolt12"],"idempotency_key":Uuid::new_v4(),"maximum_fee_msat":1000});
    let pay = format!("{wallet}/payments/submissions");
    assert_eq!(
        api.post(&pay, actor, &request).await.body["code"],
        "payment_approval_required"
    );
    let queue = format!("{account}/payment-approvals");
    let items = api.get(&queue, operator).await.body;
    let row = &items[0];
    let hash = row["destination"].as_str().unwrap();
    assert_status(
        &api.put(
            &format!("{queue}/{}", row["id"].as_str().unwrap()),
            operator,
            json!({"decision":"approve"}),
        )
        .await,
        StatusCode::OK,
    );
    let accepted = api.post(&pay, actor, &request).await;
    assert_status(&accepted, StatusCode::ACCEPTED);
    let result = accepted.parse::<PaymentSubmission>();
    assert_eq!(result.payment.lightning.as_ref().unwrap().payment_hash, hash);
    wait_until(
        Duration::from_secs(45),
        "the original negotiated invoice settles",
        || async {
            api.get(&result.status_url, actor)
                .await
                .parse::<PaymentSubmission>()
                .state
                == PaymentExecutionState::Settled
        },
    )
    .await;
    app.stop();
}

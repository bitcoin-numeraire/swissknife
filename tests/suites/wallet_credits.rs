use reqwest::StatusCode;
use swissknife_types::{
    CreateWalletCredit, CreditPurpose, Invoice, InvoiceStatus, Ledger, Permission, WalletCreditReceipt,
};
use uuid::Uuid;

use crate::common::{app, assert_status, Auth, TestApp};

#[tokio::test]
async fn posts_once_with_scoped_authority_and_preserves_history_after_restart() {
    let app = TestApp::isolated("wallet-credit", &[]).await;
    let token = app.admin_token().await;
    let target = app.create_account_with_wallet(token, "credit-target").await;
    let wallet = &target.wallet;
    let path = format!("/v1/wallets/{}/credits", wallet.id);
    let api = app.api();
    let amount = 9_007_199_254_740_993;
    let request = CreateWalletCredit {
        request_id: Uuid::new_v4(),
        account_id: wallet.account_id,
        asset_id: wallet.asset_id,
        amount_units: amount,
        purpose: CreditPurpose::Opening,
        reason: "Opening allocation".into(),
    };
    let editor = app.api_key(token, vec![Permission::WriteWallet]).await;
    assert_status(&api.post(&path, Auth::None, &request).await, StatusCode::UNAUTHORIZED);
    assert_status(
        &api.post(&path, Auth::ApiKey(&editor), &request).await,
        StatusCode::FORBIDDEN,
    );
    let personal = format!("/v1/me/wallets/{}/credits", wallet.id);
    assert_status(
        &api.post(&personal, Auth::ApiKey(&target.key), &request).await,
        StatusCode::NOT_FOUND,
    );

    let operator = app.api_key(token, vec![Permission::CreditWallet]).await;
    let auth = Auth::ApiKey(&operator);
    let (a, b) = tokio::join!(api.post(&path, auth, &request), api.post(&path, auth, &request));
    assert_status(&a, StatusCode::OK);
    assert_status(&b, StatusCode::OK);
    let receipt = a.parse::<WalletCreditReceipt>();
    assert_eq!(receipt, b.parse::<WalletCreditReceipt>());
    assert_eq!(receipt.request, request);
    assert!(receipt.api_key_id.is_some());
    assert_ne!(receipt.actor_account_id, wallet.account_id);
    let balance = app.wallet_balance(token, wallet.id).await;
    assert_eq!(balance.available_msat, amount as i64);
    assert_eq!(balance.available_units, Some(amount.to_string()));
    assert_eq!(balance.reserved_msat, 0);
    assert_eq!(balance.received_msat, 0);

    for conflict in [
        CreateWalletCredit {
            amount_units: amount - 1,
            ..request.clone()
        },
        CreateWalletCredit {
            request_id: Uuid::new_v4(),
            ..request.clone()
        },
    ] {
        assert_status(&api.post(&path, auth, conflict).await, StatusCode::CONFLICT);
    }
    let incoming_path = format!("/v1/me/wallets/{}/invoices", wallet.id);
    let invoices = api
        .get(&incoming_path, Auth::ApiKey(&target.key))
        .await
        .parse::<Vec<Invoice>>();
    assert_eq!(invoices.len(), 1);
    let invoice = &invoices[0];
    assert_eq!(invoice.id, receipt.invoice_id);
    assert_eq!(invoice.ledger, Ledger::Adjustment);
    assert_eq!(invoice.credit_purpose, Some(CreditPurpose::Opening));
    assert_eq!(invoice.status, InvoiceStatus::Settled);
    assert_eq!(invoice.amount_received_msat, Some(amount));
    assert_eq!(invoice.credit_amount_units, Some(amount.to_string()));
    assert_eq!(invoice.fee_msat, Some(0));
    assert!(invoice.ln_invoice.is_none() && invoice.bitcoin_output.is_none());
    let restarted = app.restart("wallet-credit-replay", &[]).await;
    let replay = restarted.api().post(&path, auth, &request).await;
    assert_status(&replay, StatusCode::OK);
    assert_eq!(replay.parse::<WalletCreditReceipt>(), receipt);
    assert_eq!(
        restarted
            .wallet_balance(restarted.admin_token().await, wallet.id)
            .await
            .available_msat,
        amount as i64
    );
}

#[tokio::test]
async fn rejects_invalid_targets_and_overflow_without_consuming_the_key() {
    let app = app().await;
    let token = app.admin_token().await;
    let target = app.create_wallet(token, "credit-range").await;
    let api = app.api();
    let path = format!("/v1/wallets/{}/credits", target.id);
    let auth = Auth::Bearer(token);
    let req = CreateWalletCredit {
        request_id: Uuid::new_v4(),
        account_id: target.account_id,
        asset_id: target.asset_id,
        amount_units: 1,
        purpose: CreditPurpose::TopUp,
        reason: "Funding".into(),
    };
    for invalid in [
        CreateWalletCredit {
            amount_units: 0,
            ..req.clone()
        },
        CreateWalletCredit {
            amount_units: u64::MAX,
            ..req.clone()
        },
        CreateWalletCredit {
            reason: "  ".into(),
            ..req.clone()
        },
    ] {
        assert_status(&api.post(&path, auth, invalid).await, StatusCode::UNPROCESSABLE_ENTITY);
    }
    for invalid in [
        CreateWalletCredit {
            account_id: Uuid::new_v4(),
            ..req.clone()
        },
        CreateWalletCredit {
            asset_id: Uuid::new_v4(),
            ..req.clone()
        },
    ] {
        assert_status(&api.post(&path, auth, invalid).await, StatusCode::NOT_FOUND);
    }
    assert_status(&api.post(&path, auth, &req).await, StatusCode::OK);
    let next = CreateWalletCredit {
        request_id: Uuid::new_v4(),
        amount_units: i64::MAX as u64,
        ..req
    };
    assert_status(&api.post(&path, auth, &next).await, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.wallet_balance(token, target.id).await.available_msat, 1);
    assert_status(
        &api.post(
            &path,
            auth,
            CreateWalletCredit {
                amount_units: 3,
                ..next
            },
        )
        .await,
        StatusCode::OK,
    );
    assert_eq!(app.wallet_balance(token, target.id).await.available_msat, 4);
}

#[tokio::test]
async fn opening_credit_can_fund_an_internal_address_payment() {
    use swissknife_types::{Payment, PaymentStatus, RegisterLnAddressRequest, SendPaymentRequest};
    let app = TestApp::isolated("credit-spend", &[("SWISSKNIFE_DOMAIN", "credits.test".into())]).await;
    let token = app.admin_token().await;
    let payer = app.create_account_with_wallet(token, "opening-payer").await;
    let payee = app.create_account_with_wallet(token, "opening-payee").await;
    let username = crate::common::fixtures::unique("receiver");
    let api = app.api();
    let address = api
        .post(
            "/v1/lightning-addresses",
            Auth::Bearer(token),
            RegisterLnAddressRequest {
                account_id: Some(payee.wallet.account_id),
                username: username.clone(),
                allows_nostr: false,
                nostr_pubkey: None,
            },
        )
        .await;
    assert_status(&address, StatusCode::OK);
    let receipt = api
        .post(
            &format!("/v1/wallets/{}/credits", payer.wallet.id),
            Auth::Bearer(token),
            CreateWalletCredit {
                request_id: Uuid::new_v4(),
                account_id: payer.wallet.account_id,
                asset_id: payer.wallet.asset_id,
                amount_units: 1_000_000_000,
                purpose: CreditPurpose::Opening,
                reason: "Operator opening balance".into(),
            },
        )
        .await;
    assert_status(&receipt, StatusCode::OK);
    let pay = api
        .post(
            &format!("/v1/me/wallets/{}/payments", payer.wallet.id),
            Auth::ApiKey(&payer.key),
            SendPaymentRequest {
                wallet_id: None,
                input: format!("{username}@credits.test"),
                amount_msat: Some(100_000_000),
                comment: None,
            },
        )
        .await;
    assert_status(&pay, StatusCode::OK);
    let payment = pay.parse::<Payment>();
    assert_eq!(payment.ledger, Ledger::Internal);
    assert_eq!(payment.status, PaymentStatus::Settled);
    assert_eq!(
        app.wallet_balance(token, payer.wallet.id).await.available_msat,
        900_000_000
    );
    assert_eq!(
        app.wallet_balance(token, payee.wallet.id).await.available_msat,
        100_000_000
    );
}

#[tokio::test]
async fn oauth2_credit_requires_an_explicit_access_token_scope() {
    use crate::common::oauth2::{oauth2_app, CLIENT_FULL, CLIENT_READONLY};
    use swissknife_types::Wallet;
    let app = oauth2_app().await;
    let operator = app.token(CLIENT_FULL).await;
    let read_only = app.token(CLIENT_READONLY).await;
    let api = app.api();
    let wallets = api
        .get("/v1/me/wallets", Auth::Bearer(&operator))
        .await
        .parse::<Vec<Wallet>>();
    let target = wallets.first().expect("OAuth2 wallet");
    let path = format!("/v1/wallets/{}/credits", target.id);
    let request = CreateWalletCredit {
        request_id: Uuid::new_v4(),
        account_id: target.account_id,
        asset_id: target.asset_id,
        amount_units: 27,
        purpose: CreditPurpose::TopUp,
        reason: "OAuth2 operator credit".into(),
    };
    assert_status(
        &api.post(&path, Auth::Bearer(&read_only), &request).await,
        StatusCode::FORBIDDEN,
    );
    let response = api.post(&path, Auth::Bearer(&operator), &request).await;
    assert_status(&response, StatusCode::OK);
    let receipt = response.parse::<WalletCreditReceipt>();
    assert_eq!(receipt.actor_account_id, target.account_id);
    assert!(receipt.api_key_id.is_none());
}

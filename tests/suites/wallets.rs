//! `/v1/wallets` — admin wallet management. Permission-gated (`*:wallet`).

use reqwest::StatusCode;
use serde_json::json;

use swissknife_types::{
    Balance, BtcAddress, BtcAddressType, CreateWalletRequest, Ledger, LnAddress, NewBtcAddressRequest, Payment,
    PaymentStatus, Permission, RegisterLnAddressRequest, SendPaymentRequest, Wallet, WalletOverview,
};

use crate::common::fixtures::unique;
use crate::common::{app, assert_error, assert_status, Auth};

mod register_wallet {
    use super::*;

    #[tokio::test]
    async fn succeeds_and_is_persisted() {
        let app = app().await;
        let token = app.admin_token().await;
        let existing = app.create_wallet(token, "wallet-create-owner").await;

        let res = app
            .api()
            .post(
                "/v1/wallets",
                Auth::Bearer(token),
                CreateWalletRequest {
                    account_id: Some(existing.account_id),
                    asset_id: existing.asset_id,
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let wallet = res.parse::<Wallet>();
        assert_eq!(wallet.account_id, existing.account_id);
        assert_eq!(wallet.asset_id, existing.asset_id);
        assert_eq!(wallet.balance.available_msat, 0);

        // Persisted: fetchable by id.
        let got = app
            .api()
            .get(&format!("/v1/wallets/{}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&got, StatusCode::OK);
        assert_eq!(got.parse::<Wallet>().id, wallet.id);
    }

    #[tokio::test]
    async fn requires_authentication() {
        let app = app().await;
        let res = app
            .api()
            .post(
                "/v1/wallets",
                Auth::None,
                CreateWalletRequest {
                    account_id: Some(uuid::Uuid::new_v4()),
                    asset_id: uuid::Uuid::new_v4(),
                },
            )
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_a_malformed_body() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app.api().post("/v1/wallets", Auth::Bearer(token), json!({})).await;
        assert_error(&res, StatusCode::BAD_REQUEST);
    }
}

mod get_wallet {
    use super::*;

    #[tokio::test]
    async fn includes_bitcoin_addresses_for_read_wallet_permission() {
        let app = app().await;
        let admin_token = app.admin_token().await;
        let wallet = app.create_wallet(admin_token, "wallet-details-addresses").await;

        let created = app
            .api()
            .post(
                "/v1/bitcoin/addresses",
                Auth::Bearer(admin_token),
                NewBtcAddressRequest {
                    wallet_id: Some(wallet.id),
                    address_type: Some(BtcAddressType::P2tr),
                },
            )
            .await;
        assert_status(&created, StatusCode::OK);
        let created = created.parse::<BtcAddress>();

        let read_wallet_key = app.api_key(admin_token, vec![Permission::ReadWallet]).await;
        let res = app
            .api()
            .get(&format!("/v1/wallets/{}", wallet.id), Auth::ApiKey(&read_wallet_key))
            .await;
        assert_status(&res, StatusCode::OK);
        let detailed = res.parse::<Wallet>();

        assert!(
            detailed.btc_addresses.iter().any(|address| address.id == created.id),
            "wallet details include the generated Bitcoin address"
        );
    }

    #[tokio::test]
    async fn unknown_id_is_not_found() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app
            .api()
            .get(&format!("/v1/wallets/{}", uuid::Uuid::new_v4()), Auth::Bearer(token))
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }
}

mod list {
    use super::*;

    #[tokio::test]
    async fn includes_the_registered_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "wallet-list").await;

        // A large limit keeps the wallet on the page regardless of how many
        // others share the instance.
        let res = app.api().get("/v1/wallets?limit=1000", Auth::Bearer(token)).await;
        assert_status(&res, StatusCode::OK);
        assert!(
            res.parse::<Vec<Wallet>>().iter().any(|w| w.id == wallet.id),
            "the registered wallet is listed"
        );
    }
}

mod list_overviews {
    use super::*;

    fn register_ln_address(wallet: &Wallet, username: &str) -> RegisterLnAddressRequest {
        RegisterLnAddressRequest {
            account_id: Some(wallet.account_id),
            username: username.to_string(),
            allows_nostr: false,
            nostr_pubkey: None,
        }
    }

    fn internal_payment(wallet_id: uuid::Uuid, input: String, amount_msat: u64) -> SendPaymentRequest {
        SendPaymentRequest {
            wallet_id: Some(wallet_id),
            input,
            amount_msat: Some(amount_msat),
            comment: None,
        }
    }

    fn find_overview(overviews: &[WalletOverview], id: uuid::Uuid) -> &WalletOverview {
        overviews
            .iter()
            .find(|overview| overview.id == id)
            .expect("created wallet is present in overviews")
    }

    fn assert_balance(overview: &WalletOverview, balance: &Balance) {
        assert_eq!(overview.balance.received_msat, balance.received_msat);
        assert_eq!(overview.balance.sent_msat, balance.sent_msat);
        assert_eq!(overview.balance.fees_paid_msat, balance.fees_paid_msat);
        assert_eq!(overview.balance.reserved_msat, balance.reserved_msat);
        assert_eq!(overview.balance.available_msat, balance.available_msat);
    }

    #[tokio::test]
    async fn includes_aggregated_wallet_activity() {
        let app = app().await;
        let token = app.admin_token().await;
        let payer = app.create_wallet(token, "wallet-overview-payer").await;
        let payee = app.create_wallet(token, "wallet-overview-payee").await;
        let username = unique("walletoverview");
        let address_input = format!("{username}@localhost");
        let amount_msat = 25_000_000u64;

        app.fund_onchain(token, payer.id, 100_000).await;

        let address = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(token),
                register_ln_address(&payee, &username),
            )
            .await;
        assert_status(&address, StatusCode::OK);
        let address = address.parse::<LnAddress>();

        let payment = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                internal_payment(payer.id, address_input, amount_msat),
            )
            .await;
        assert_status(&payment, StatusCode::OK);
        let payment = payment.parse::<Payment>();
        assert_eq!(payment.status, PaymentStatus::Settled);
        assert_eq!(payment.ledger, Ledger::Internal);

        let payer_balance = app.wallet_balance(token, payer.id).await;
        let payee_balance = app.wallet_balance(token, payee.id).await;

        let res = app.api().get("/v1/wallets/overviews", Auth::Bearer(token)).await;
        assert_status(&res, StatusCode::OK);
        let overviews = res.parse::<Vec<WalletOverview>>();

        let payer_overview = find_overview(&overviews, payer.id);
        assert_balance(payer_overview, &payer_balance);
        assert_eq!(payer_overview.n_payments, 1);
        assert_eq!(payer_overview.n_contacts, 1);
        assert!(payer_overview.ln_address.is_none());

        let payee_overview = find_overview(&overviews, payee.id);
        assert_balance(payee_overview, &payee_balance);
        assert_eq!(payee_overview.n_payments, 0);
        assert_eq!(payee_overview.n_invoices, 1);
        assert_eq!(payee_overview.n_contacts, 0);
        assert_eq!(
            payee_overview
                .ln_address
                .as_ref()
                .expect("registered address is included")
                .id,
            address.id
        );
    }
}

mod delete {
    use super::*;

    #[tokio::test]
    async fn bulk_delete_requires_an_effective_filter() {
        let app = crate::common::TestApp::isolated("wallet-delete-filter", &[]).await;
        let token = app.admin_token().await;
        let first = app.create_wallet(token, "wallet-filter-first").await;
        let second = app.create_wallet(token, "wallet-filter-second").await;

        for query in ["", "?limit=1", "?offset=1", "?order_direction=Asc", "?unknown=1"] {
            let response = app
                .api()
                .delete(&format!("/v1/wallets{query}"), Auth::Bearer(token))
                .await;
            assert_error(&response, StatusCode::BAD_REQUEST);
            for id in [first.id, second.id] {
                let retained = app.api().get(&format!("/v1/wallets/{id}"), Auth::Bearer(token)).await;
                assert_status(&retained, StatusCode::OK);
                assert_eq!(retained.parse::<Wallet>().id, id);
            }
        }

        let listed = app.api().get("/v1/wallets", Auth::Bearer(token)).await;
        assert_status(&listed, StatusCode::OK);
        assert!(listed.parse::<Vec<Wallet>>().iter().any(|wallet| wallet.id == first.id));

        let deleted = app
            .api()
            .delete(
                &format!("/v1/wallets?account_id={}", first.account_id),
                Auth::Bearer(token),
            )
            .await;
        assert_status(&deleted, StatusCode::OK);
        assert_eq!(deleted.parse::<u64>(), 1);
        let missing = app
            .api()
            .get(&format!("/v1/wallets/{}", first.id), Auth::Bearer(token))
            .await;
        assert_error(&missing, StatusCode::NOT_FOUND);
        let retained = app
            .api()
            .get(&format!("/v1/wallets/{}", second.id), Auth::Bearer(token))
            .await;
        assert_status(&retained, StatusCode::OK);
    }

    #[tokio::test]
    async fn deletes_a_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "wallet-del").await;

        let del = app
            .api()
            .delete(&format!("/v1/wallets/{}", wallet.id), Auth::Bearer(token))
            .await;
        assert_status(&del, StatusCode::OK);

        let gone = app
            .api()
            .get(&format!("/v1/wallets/{}", wallet.id), Auth::Bearer(token))
            .await;
        assert_error(&gone, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn wallet_identity_enrollment_is_scoped_and_rejects_concurrent_replacement() {
    let app = app().await;
    let admin = app.admin_token().await;
    let owner = app.create_account_with_wallet(admin, "identity-owner").await;
    let other = app.create_account_with_wallet(admin, "identity-other").await;
    let auth = Auth::ApiKey(&owner.key);
    let api = app.api();
    let path = format!("/v1/me/wallets/{}/identity", owner.wallet.id);
    let first = "17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917";
    let second = "d41b22899549e1f3d335a31002cfd382174006e166d3e658e3a5eecdb6463573";
    let thirds = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    let initial = json!({"recovery_pubkey":first,"nostr_pubkey":first});
    let setup = json!({"identity": initial});
    assert_error(&api.get(&path, Auth::None).await, StatusCode::UNAUTHORIZED);
    assert_error(
        &api.put(&path, Auth::ApiKey(&other.key), &setup).await,
        StatusCode::NOT_FOUND,
    );
    assert!(api.get(&path, auth).await.body.is_null());
    let wrong = json!({"identity":{"recovery_pubkey": first,"nostr_pubkey": "ff".repeat(32)}});
    assert_error(&api.put(&path, auth, wrong).await, StatusCode::UNPROCESSABLE_ENTITY);
    let invalid = api
        .put(&path, auth, json!({"identity":initial,"mnemonic":"not accepted"}))
        .await;
    assert!(invalid.status.is_client_error());
    for _ in 0..2 {
        assert_eq!(api.put(&path, auth, &setup).await.body, initial);
    }
    let a = json!({"identity":{"recovery_pubkey":first,"nostr_pubkey":second},"expected":initial});
    let b = json!({"identity":{"recovery_pubkey":first,"nostr_pubkey":thirds},"expected":initial});
    let (ra, rb) = tokio::join!(api.put(&path, auth, &a), api.put(&path, auth, &b));
    let mut statuses = [ra.status.as_u16(), rb.status.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    assert_eq!(
        api.get(&path, auth).await.body,
        if ra.status.is_success() {
            a["identity"].clone()
        } else {
            b["identity"].clone()
        }
    );
    assert_error(&api.put(&path, auth, setup).await, StatusCode::CONFLICT);
    assert_error(&api.get(&path, Auth::ApiKey(&other.key)).await, StatusCode::NOT_FOUND);
}

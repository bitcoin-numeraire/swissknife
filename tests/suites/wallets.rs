//! `/v1/wallets` — admin wallet management. Permission-gated (`*:wallet`).

use reqwest::StatusCode;
use serde_json::json;

use swissknife_types::{
    Balance, BtcAddress, BtcAddressType, Ledger, LnAddress, NewBtcAddressRequest, Payment, PaymentStatus, Permission,
    RegisterLnAddressRequest, RegisterWalletRequest, SendPaymentRequest, Wallet, WalletOverview,
};

use crate::common::fixtures::unique;
use crate::common::{app, assert_error, assert_status, Auth};

mod register_wallet {
    use super::*;

    #[tokio::test]
    async fn succeeds_and_is_persisted() {
        let app = app().await;
        let token = app.admin_token().await;
        let user_id = unique("wallet-user");

        let res = app
            .api()
            .post(
                "/v1/wallets",
                Auth::Bearer(token),
                RegisterWalletRequest {
                    user_id: user_id.clone(),
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let wallet = res.parse::<Wallet>();
        assert_eq!(wallet.user_id, user_id);
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
                RegisterWalletRequest {
                    user_id: unique("wallet-user"),
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

        let read_wallet_key = app
            .user_api_key(admin_token, &wallet.user_id, vec![Permission::ReadWallet])
            .await;
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

    fn register_ln_address(wallet_id: uuid::Uuid, username: &str) -> RegisterLnAddressRequest {
        RegisterLnAddressRequest {
            wallet_id: Some(wallet_id),
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
                register_ln_address(payee.id, &username),
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

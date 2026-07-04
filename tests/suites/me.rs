//! `/v1/me/*` — the account-scoped user surface. Authenticated callers get an
//! account profile at `/v1/me`; money-moving operations require an explicit
//! account-owned wallet in the path.

use reqwest::StatusCode;
use serde_json::json;

use swissknife_types::{
    Account, AccountPreferences, ApiKey, Balance, BtcAddress, Contact, CreateApiKeyRequest, Invoice, LnAddress,
    NewBtcAddressRequest, NewInvoiceRequest, Payment, PaymentStatus, Permission, RegisterLnAddressRequest,
    SendPaymentRequest, UpdateAccountPreferencesRequest, UpdateLnAddressRequest, Wallet,
};

use crate::common::fixtures::unique;
use crate::common::{app, assert_error, assert_status, Auth};

mod account {
    use super::*;

    #[tokio::test]
    async fn returns_the_callers_account_profile() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-profile").await;

        let res = app.api().get("/v1/me", Auth::ApiKey(&user.key)).await;
        assert_status(&res, StatusCode::OK);
        let profile = res.parse::<Account>();

        assert_eq!(profile.id, user.wallet.account_id);
        assert!(profile
            .permissions
            .expect("profile exposes effective permissions")
            .contains(&Permission::ReadWallet));
    }

    #[tokio::test]
    async fn lists_only_the_callers_wallets() {
        let app = app().await;
        let token = app.admin_token().await;
        let alice = app.create_user(token, "me-wallet-a").await;
        let bob = app.create_user(token, "me-wallet-b").await;

        let res = app.api().get("/v1/me/wallets", Auth::ApiKey(&alice.key)).await;
        assert_status(&res, StatusCode::OK);
        let wallets = res.parse::<Vec<Wallet>>();

        assert!(wallets.iter().any(|wallet| wallet.id == alice.wallet.id));
        assert!(!wallets.iter().any(|wallet| wallet.id == bob.wallet.id));
        assert!(wallets
            .iter()
            .all(|wallet| wallet.account_id == alice.wallet.account_id));
    }

    #[tokio::test]
    async fn gets_wallet_and_balance_by_path() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-wallet-get").await;

        let wallet = app
            .api()
            .get(&format!("/v1/me/wallets/{}", user.wallet.id), Auth::ApiKey(&user.key))
            .await;
        assert_status(&wallet, StatusCode::OK);
        assert_eq!(wallet.parse::<Wallet>().id, user.wallet.id);

        let balance = app
            .api()
            .get(
                &format!("/v1/me/wallets/{}/balance", user.wallet.id),
                Auth::ApiKey(&user.key),
            )
            .await;
        assert_status(&balance, StatusCode::OK);
        assert_eq!(balance.parse::<Balance>().available_msat, 0);
    }

    #[tokio::test]
    async fn rejects_another_accounts_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let alice = app.create_user(token, "me-wallet-own-a").await;
        let bob = app.create_user(token, "me-wallet-own-b").await;

        let res = app
            .api()
            .get(&format!("/v1/me/wallets/{}", bob.wallet.id), Auth::ApiKey(&alice.key))
            .await;

        assert_error(&res, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn requires_authentication() {
        let app = app().await;
        let res = app.api().get("/v1/me", Auth::None).await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn works_without_any_permission() {
        let app = app().await;
        let token = app.admin_token().await;
        let _subject = unique("me-noperm");
        let key = app.user_api_key(token, uuid::Uuid::new_v4(), vec![]).await;

        let res = app.api().get("/v1/me", Auth::ApiKey(&key)).await;
        assert_status(&res, StatusCode::OK);
        assert!(res.parse::<Account>().permissions.unwrap_or_default().is_empty());
    }
}

mod preferences {
    use super::*;

    #[tokio::test]
    async fn get_and_replace_dashboard_settings() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-prefs").await;

        let before = app.api().get("/v1/me/preferences", Auth::ApiKey(&user.key)).await;
        assert_status(&before, StatusCode::OK);
        assert_eq!(before.parse::<AccountPreferences>().dashboard_settings, json!({}));

        let update = app
            .api()
            .put(
                "/v1/me/preferences",
                Auth::ApiKey(&user.key),
                UpdateAccountPreferencesRequest {
                    dashboard_settings: json!({ "version": 1, "wallet": { "density": "compact" } }),
                },
            )
            .await;
        assert_status(&update, StatusCode::OK);
        assert_eq!(
            update.parse::<AccountPreferences>().dashboard_settings,
            json!({ "version": 1, "wallet": { "density": "compact" } })
        );
    }
}

mod bitcoin {
    use super::*;

    #[tokio::test]
    async fn generates_and_lists_deposit_addresses_for_the_path_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-btc").await;

        let created = app
            .api()
            .post(
                &format!("/v1/me/wallets/{}/bitcoin/addresses", user.wallet.id),
                Auth::ApiKey(&user.key),
                NewBtcAddressRequest {
                    wallet_id: None,
                    address_type: None,
                },
            )
            .await;
        assert_status(&created, StatusCode::OK);
        let created = created.parse::<BtcAddress>();
        assert_eq!(created.wallet_id, user.wallet.id);

        let list = app
            .api()
            .get(
                &format!("/v1/me/wallets/{}/bitcoin/addresses", user.wallet.id),
                Auth::ApiKey(&user.key),
            )
            .await;
        assert_status(&list, StatusCode::OK);
        assert!(list
            .parse::<Vec<BtcAddress>>()
            .iter()
            .any(|address| address.id == created.id));
    }

    #[tokio::test]
    async fn rejects_another_accounts_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let alice = app.create_user(token, "me-btc-a").await;
        let bob = app.create_user(token, "me-btc-b").await;

        let res = app
            .api()
            .post(
                &format!("/v1/me/wallets/{}/bitcoin/addresses", bob.wallet.id),
                Auth::ApiKey(&alice.key),
                NewBtcAddressRequest {
                    wallet_id: None,
                    address_type: None,
                },
            )
            .await;

        assert_error(&res, StatusCode::NOT_FOUND);
    }
}

mod invoices {
    use super::*;

    #[tokio::test]
    async fn generates_lists_and_gets_for_the_path_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-inv").await;

        let created = app
            .api()
            .post(
                &format!("/v1/me/wallets/{}/invoices", user.wallet.id),
                Auth::ApiKey(&user.key),
                NewInvoiceRequest {
                    wallet_id: None,
                    amount_msat: 21_000,
                    description: None,
                    expiry: None,
                },
            )
            .await;
        assert_status(&created, StatusCode::OK);
        let created = created.parse::<Invoice>();
        assert_eq!(created.wallet_id, user.wallet.id);

        let list = app
            .api()
            .get(
                &format!("/v1/me/wallets/{}/invoices", user.wallet.id),
                Auth::ApiKey(&user.key),
            )
            .await;
        assert!(list
            .parse::<Vec<Invoice>>()
            .iter()
            .any(|invoice| invoice.id == created.id));

        let got = app
            .api()
            .get(
                &format!("/v1/me/wallets/{}/invoices/{}", user.wallet.id, created.id),
                Auth::ApiKey(&user.key),
            )
            .await;
        assert_status(&got, StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_another_accounts_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let alice = app.create_user(token, "me-inv-a").await;
        let bob = app.create_user(token, "me-inv-b").await;

        let res = app
            .api()
            .post(
                &format!("/v1/me/wallets/{}/invoices", bob.wallet.id),
                Auth::ApiKey(&alice.key),
                NewInvoiceRequest {
                    wallet_id: None,
                    amount_msat: 1_000,
                    description: None,
                    expiry: None,
                },
            )
            .await;

        assert_error(&res, StatusCode::NOT_FOUND);
    }
}

mod payments {
    use super::*;

    #[tokio::test]
    async fn rejects_an_unparseable_input() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-pay-bad").await;

        let res = app
            .api()
            .post(
                &format!("/v1/me/wallets/{}/payments", user.wallet.id),
                Auth::ApiKey(&user.key),
                SendPaymentRequest {
                    wallet_id: None,
                    input: "notapaymentinput".to_string(),
                    amount_msat: None,
                    comment: None,
                },
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn settles_internally_and_is_isolated() {
        let app = app().await;
        let token = app.admin_token().await;
        let payer = app.create_user(token, "me-pay-src").await;
        let payee = app.create_user(token, "me-pay-dst").await;
        let other = app.create_user(token, "me-pay-other").await;

        app.fund_onchain(token, payer.wallet.id, 200_000).await;

        let bolt11 = app
            .api()
            .post(
                &format!("/v1/me/wallets/{}/invoices", payee.wallet.id),
                Auth::ApiKey(&payee.key),
                NewInvoiceRequest {
                    wallet_id: None,
                    amount_msat: 50_000_000,
                    description: None,
                    expiry: None,
                },
            )
            .await
            .parse::<Invoice>()
            .ln_invoice
            .expect("a bolt11 invoice")
            .bolt11;

        let res = app
            .api()
            .post(
                &format!("/v1/me/wallets/{}/payments", payer.wallet.id),
                Auth::ApiKey(&payer.key),
                SendPaymentRequest {
                    wallet_id: None,
                    input: bolt11,
                    amount_msat: None,
                    comment: None,
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let payment = res.parse::<Payment>();
        assert_eq!(payment.status, PaymentStatus::Settled);

        let payer_list = app
            .api()
            .get(
                &format!("/v1/me/wallets/{}/payments", payer.wallet.id),
                Auth::ApiKey(&payer.key),
            )
            .await;
        assert!(payer_list.parse::<Vec<Payment>>().iter().any(|p| p.id == payment.id));
        let other_list = app
            .api()
            .get(
                &format!("/v1/me/wallets/{}/payments", other.wallet.id),
                Auth::ApiKey(&other.key),
            )
            .await;
        assert!(!other_list.parse::<Vec<Payment>>().iter().any(|p| p.id == payment.id));

        let payer_get = app
            .api()
            .get(
                &format!("/v1/me/wallets/{}/payments/{}", payer.wallet.id, payment.id),
                Auth::ApiKey(&payer.key),
            )
            .await;
        assert_status(&payer_get, StatusCode::OK);
    }
}

mod ln_address {
    use super::*;

    #[tokio::test]
    async fn register_get_update_delete() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-lnaddr").await;
        let username = unique("me-lnaddr");

        let before = app.api().get("/v1/me/lightning-address", Auth::ApiKey(&user.key)).await;
        assert_status(&before, StatusCode::OK);
        assert!(before.parse::<Option<LnAddress>>().is_none());

        let reg = app
            .api()
            .post(
                "/v1/me/lightning-address",
                Auth::ApiKey(&user.key),
                RegisterLnAddressRequest {
                    account_id: None,
                    username: username.clone(),
                    allows_nostr: false,
                    nostr_pubkey: None,
                },
            )
            .await;
        assert_status(&reg, StatusCode::OK);
        assert_eq!(reg.parse::<LnAddress>().username, username);

        let got = app.api().get("/v1/me/lightning-address", Auth::ApiKey(&user.key)).await;
        assert_eq!(got.parse::<Option<LnAddress>>().expect("registered").username, username);

        let updated = app
            .api()
            .put(
                "/v1/me/lightning-address",
                Auth::ApiKey(&user.key),
                UpdateLnAddressRequest {
                    username: None,
                    active: Some(false),
                    allows_nostr: None,
                    nostr_pubkey: None,
                },
            )
            .await;
        assert_status(&updated, StatusCode::OK);
        assert!(!updated.parse::<LnAddress>().active);

        let del = app
            .api()
            .delete("/v1/me/lightning-address", Auth::ApiKey(&user.key))
            .await;
        assert_status(&del, StatusCode::OK);
        let after = app.api().get("/v1/me/lightning-address", Auth::ApiKey(&user.key)).await;
        assert!(after.parse::<Option<LnAddress>>().is_none());
    }
}

mod contacts {
    use super::*;

    #[tokio::test]
    async fn lists_contacts_for_the_path_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-contacts").await;

        let res = app
            .api()
            .get(
                &format!("/v1/me/wallets/{}/contacts", user.wallet.id),
                Auth::ApiKey(&user.key),
            )
            .await;
        assert_status(&res, StatusCode::OK);
        assert!(res.parse::<Vec<Contact>>().is_empty());
    }
}

mod api_keys {
    use super::*;

    #[tokio::test]
    async fn create_list_get_revoke() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-keys").await;

        let created = app
            .api()
            .post(
                "/v1/me/api-keys",
                Auth::ApiKey(&user.key),
                CreateApiKeyRequest {
                    account_id: None,
                    name: unique("me-key"),
                    permissions: vec![Permission::ReadWallet],
                    description: None,
                    expiry: None,
                },
            )
            .await;
        assert_status(&created, StatusCode::OK);
        let created = created.parse::<ApiKey>();
        assert_eq!(created.account_id, user.wallet.account_id);

        let list = app.api().get("/v1/me/api-keys", Auth::ApiKey(&user.key)).await;
        assert!(list.parse::<Vec<ApiKey>>().iter().any(|key| key.id == created.id));

        let got = app
            .api()
            .get(&format!("/v1/me/api-keys/{}", created.id), Auth::ApiKey(&user.key))
            .await;
        assert_status(&got, StatusCode::OK);

        let del = app
            .api()
            .delete(&format!("/v1/me/api-keys/{}", created.id), Auth::ApiKey(&user.key))
            .await;
        assert_status(&del, StatusCode::OK);
        let gone = app
            .api()
            .get(&format!("/v1/me/api-keys/{}", created.id), Auth::ApiKey(&user.key))
            .await;
        assert_error(&gone, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn is_isolated_between_users() {
        let app = app().await;
        let token = app.admin_token().await;
        let alice = app.create_user(token, "me-keys-a").await;
        let bob = app.create_user(token, "me-keys-b").await;

        let created = app
            .api()
            .post(
                "/v1/me/api-keys",
                Auth::ApiKey(&alice.key),
                CreateApiKeyRequest {
                    account_id: None,
                    name: unique("me-key-a"),
                    permissions: vec![],
                    description: None,
                    expiry: None,
                },
            )
            .await
            .parse::<ApiKey>();

        let bob_list = app.api().get("/v1/me/api-keys", Auth::ApiKey(&bob.key)).await;
        assert!(!bob_list.parse::<Vec<ApiKey>>().iter().any(|key| key.id == created.id));
        let bob_get = app
            .api()
            .get(&format!("/v1/me/api-keys/{}", created.id), Auth::ApiKey(&bob.key))
            .await;
        assert_error(&bob_get, StatusCode::NOT_FOUND);
    }
}

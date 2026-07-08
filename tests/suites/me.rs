//! `/v1/me/*` — the user-scoped surface. Auth only (no permission check),
//! everything scoped to the caller's own wallet. This is what real users hit, so
//! it is tested as an external client with no knowledge that the use cases are
//! shared with the admin endpoints: every action works without naming a wallet,
//! results are isolated per user, and a `wallet_id` in a body is ignored (a user
//! can never touch another wallet).

use swissknife_types::{
    ApiKey, Balance, BtcAddress, Contact, CreateApiKeyRequest, Invoice, LnAddress, NewBtcAddressRequest,
    NewInvoiceRequest, Payment, PaymentStatus, Permission, RegisterLnAddressRequest, SendPaymentRequest,
    UpdateLnAddressRequest, Wallet,
};

use reqwest::StatusCode;

use crate::common::fixtures::unique;
use crate::common::{app, assert_error, assert_status, Auth};

mod wallet {
    use super::*;

    #[tokio::test]
    async fn returns_the_callers_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-wallet").await;

        let res = app.api().get("/v1/me", Auth::ApiKey(&user.key)).await;
        assert_status(&res, StatusCode::OK);
        let wallet = res.parse::<Wallet>();
        assert_eq!(wallet.id, user.wallet.id);
        assert_eq!(wallet.account_id, user.wallet.account_id);
        assert_eq!(wallet.asset_id, user.wallet.asset_id);
    }

    #[tokio::test]
    async fn is_isolated_between_users() {
        let app = app().await;
        let token = app.admin_token().await;
        let alice = app.create_user(token, "me-alice").await;
        let bob = app.create_user(token, "me-bob").await;

        let a = app.api().get("/v1/me", Auth::ApiKey(&alice.key)).await;
        let b = app.api().get("/v1/me", Auth::ApiKey(&bob.key)).await;
        assert_eq!(a.parse::<Wallet>().id, alice.wallet.id);
        assert_eq!(b.parse::<Wallet>().id, bob.wallet.id);
        assert_ne!(alice.wallet.id, bob.wallet.id);
    }

    #[tokio::test]
    async fn balance_is_zero_for_a_fresh_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-balance").await;

        let res = app.api().get("/v1/me/balance", Auth::ApiKey(&user.key)).await;
        assert_status(&res, StatusCode::OK);
        assert_eq!(res.parse::<Balance>().available_msat, 0);
    }

    #[tokio::test]
    async fn requires_authentication() {
        let app = app().await;
        let res = app.api().get("/v1/me", Auth::None).await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn works_without_any_permission() {
        // /me is auth-only: a key carrying no scopes still works for its wallet.
        let app = app().await;
        let token = app.admin_token().await;
        let _subject = unique("me-noperm");
        let key = app.user_api_key(token, uuid::Uuid::new_v4(), vec![]).await;

        let res = app.api().get("/v1/me", Auth::ApiKey(&key)).await;
        assert_status(&res, StatusCode::OK);
        assert_ne!(res.parse::<Wallet>().id, uuid::Uuid::nil());
    }
}

mod bitcoin {
    use super::*;

    #[tokio::test]
    async fn generates_a_deposit_address() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-btc").await;

        let res = app
            .api()
            .post(
                "/v1/me/bitcoin/address",
                Auth::ApiKey(&user.key),
                NewBtcAddressRequest {
                    wallet_id: None,
                    address_type: None,
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        assert!(!res.parse::<BtcAddress>().address.is_empty());
    }

    #[tokio::test]
    async fn list_is_scoped_to_the_caller_even_with_a_wallet_id_filter() {
        let app = app().await;
        let token = app.admin_token().await;
        let alice = app.create_user(token, "me-btc-list-a").await;
        let bob = app.create_user(token, "me-btc-list-b").await;
        let alice_wallet = alice.wallet;
        let bob_wallet = bob.wallet;
        let alice_key = alice.key;
        let bob_key = bob.key;

        let alice_address = app
            .api()
            .post(
                "/v1/me/bitcoin/address",
                Auth::ApiKey(&alice_key),
                NewBtcAddressRequest {
                    wallet_id: Some(bob_wallet.id),
                    address_type: None,
                },
            )
            .await
            .parse::<BtcAddress>();
        let bob_address = app
            .api()
            .post(
                "/v1/me/bitcoin/address",
                Auth::ApiKey(&bob_key),
                NewBtcAddressRequest {
                    wallet_id: Some(alice_wallet.id),
                    address_type: None,
                },
            )
            .await
            .parse::<BtcAddress>();

        assert_eq!(alice_address.wallet_id, alice_wallet.id);
        assert_eq!(bob_address.wallet_id, bob_wallet.id);

        let alice_list = app
            .api()
            .get(
                &format!("/v1/me/bitcoin/addresses?wallet_id={}", bob_wallet.id),
                Auth::ApiKey(&alice_key),
            )
            .await;
        assert_status(&alice_list, StatusCode::OK);
        let alice_addresses = alice_list.parse::<Vec<BtcAddress>>();

        assert!(
            alice_addresses.iter().any(|address| address.id == alice_address.id),
            "alice sees her own address"
        );
        assert!(
            !alice_addresses.iter().any(|address| address.id == bob_address.id),
            "alice cannot use wallet_id to list bob's address"
        );
        assert!(
            alice_addresses
                .iter()
                .all(|address| address.wallet_id == alice_wallet.id),
            "all listed addresses belong to alice"
        );
    }
}

mod invoices {
    use super::*;

    #[tokio::test]
    async fn generates_for_the_callers_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-inv").await;

        let res = app
            .api()
            .post(
                "/v1/me/invoices",
                Auth::ApiKey(&user.key),
                NewInvoiceRequest {
                    wallet_id: None,
                    amount_msat: 21_000,
                    description: None,
                    expiry: None,
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        assert_eq!(res.parse::<Invoice>().wallet_id, user.wallet.id);
    }

    #[tokio::test]
    async fn ignores_a_wallet_id_in_the_body() {
        // A user cannot target another wallet: the body wallet_id is ignored.
        let app = app().await;
        let token = app.admin_token().await;
        let alice = app.create_user(token, "me-inv-a").await;
        let bob = app.create_user(token, "me-inv-b").await;

        let res = app
            .api()
            .post(
                "/v1/me/invoices",
                Auth::ApiKey(&alice.key),
                NewInvoiceRequest {
                    wallet_id: Some(bob.wallet.id),
                    amount_msat: 1_000,
                    description: None,
                    expiry: None,
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        assert_eq!(
            res.parse::<Invoice>().wallet_id,
            alice.wallet.id,
            "scoped to the caller, not the wallet_id in the body"
        );
    }

    #[tokio::test]
    async fn list_and_get_are_isolated() {
        let app = app().await;
        let token = app.admin_token().await;
        let alice = app.create_user(token, "me-inv-list-a").await;
        let bob = app.create_user(token, "me-inv-list-b").await;

        let created = app
            .api()
            .post(
                "/v1/me/invoices",
                Auth::ApiKey(&alice.key),
                NewInvoiceRequest {
                    wallet_id: None,
                    amount_msat: 5_000,
                    description: None,
                    expiry: None,
                },
            )
            .await
            .parse::<Invoice>();

        let alice_list = app.api().get("/v1/me/invoices", Auth::ApiKey(&alice.key)).await;
        assert!(alice_list.parse::<Vec<Invoice>>().iter().any(|i| i.id == created.id));
        let bob_list = app.api().get("/v1/me/invoices", Auth::ApiKey(&bob.key)).await;
        assert!(
            !bob_list.parse::<Vec<Invoice>>().iter().any(|i| i.id == created.id),
            "bob cannot see alice's invoice"
        );

        let alice_get = app
            .api()
            .get(&format!("/v1/me/invoices/{}", created.id), Auth::ApiKey(&alice.key))
            .await;
        assert_status(&alice_get, StatusCode::OK);
        let bob_get = app
            .api()
            .get(&format!("/v1/me/invoices/{}", created.id), Auth::ApiKey(&bob.key))
            .await;
        assert_error(&bob_get, StatusCode::NOT_FOUND);
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
                "/v1/me/payments",
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

        // Payee issues an invoice via /me; payer pays it via /me (internal).
        let bolt11 = app
            .api()
            .post(
                "/v1/me/invoices",
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
                "/v1/me/payments",
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

        // The payment belongs to the payer only.
        let payer_list = app.api().get("/v1/me/payments", Auth::ApiKey(&payer.key)).await;
        assert!(payer_list.parse::<Vec<Payment>>().iter().any(|p| p.id == payment.id));
        let other_list = app.api().get("/v1/me/payments", Auth::ApiKey(&other.key)).await;
        assert!(
            !other_list.parse::<Vec<Payment>>().iter().any(|p| p.id == payment.id),
            "another user cannot see the payer's payment"
        );

        let payer_get = app
            .api()
            .get(&format!("/v1/me/payments/{}", payment.id), Auth::ApiKey(&payer.key))
            .await;
        assert_status(&payer_get, StatusCode::OK);
        let other_get = app
            .api()
            .get(&format!("/v1/me/payments/{}", payment.id), Auth::ApiKey(&other.key))
            .await;
        assert_error(&other_get, StatusCode::NOT_FOUND);
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
        assert!(
            before.parse::<Option<LnAddress>>().is_none(),
            "no address before registration"
        );

        let reg = app
            .api()
            .post(
                "/v1/me/lightning-address",
                Auth::ApiKey(&user.key),
                RegisterLnAddressRequest {
                    wallet_id: None,
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
        assert!(!updated.parse::<LnAddress>().active, "deactivated");

        let del = app
            .api()
            .delete("/v1/me/lightning-address", Auth::ApiKey(&user.key))
            .await;
        assert_status(&del, StatusCode::OK);
        let after = app.api().get("/v1/me/lightning-address", Auth::ApiKey(&user.key)).await;
        assert!(after.parse::<Option<LnAddress>>().is_none(), "gone after deletion");
    }

    #[tokio::test]
    async fn is_isolated_between_users() {
        let app = app().await;
        let token = app.admin_token().await;
        let alice = app.create_user(token, "me-lnaddr-a").await;
        let bob = app.create_user(token, "me-lnaddr-b").await;

        let reg = app
            .api()
            .post(
                "/v1/me/lightning-address",
                Auth::ApiKey(&alice.key),
                RegisterLnAddressRequest {
                    wallet_id: None,
                    username: unique("me-lnaddr-a"),
                    allows_nostr: false,
                    nostr_pubkey: None,
                },
            )
            .await;
        assert_status(&reg, StatusCode::OK);

        // Bob sees only his own (none), not alice's.
        let bob_get = app.api().get("/v1/me/lightning-address", Auth::ApiKey(&bob.key)).await;
        assert_status(&bob_get, StatusCode::OK);
        assert!(bob_get.parse::<Option<LnAddress>>().is_none());
    }
}

mod contacts {
    use super::*;

    #[tokio::test]
    async fn lists_contacts_for_the_caller() {
        let app = app().await;
        let token = app.admin_token().await;
        let user = app.create_user(token, "me-contacts").await;

        let res = app.api().get("/v1/me/contacts", Auth::ApiKey(&user.key)).await;
        assert_status(&res, StatusCode::OK);
        // A fresh user has no contacts; the typed parse confirms the shape.
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

        let list = app.api().get("/v1/me/api-keys", Auth::ApiKey(&user.key)).await;
        assert!(list.parse::<Vec<ApiKey>>().iter().any(|k| k.id == created.id));

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
        assert!(!bob_list.parse::<Vec<ApiKey>>().iter().any(|k| k.id == created.id));
        let bob_get = app
            .api()
            .get(&format!("/v1/me/api-keys/{}", created.id), Auth::ApiKey(&bob.key))
            .await;
        assert_error(&bob_get, StatusCode::NOT_FOUND);
    }
}

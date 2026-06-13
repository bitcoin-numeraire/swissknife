//! `/v1/lightning-addresses` — admin LN-address management, permission-gated
//! (`*:ln_address`). Each test registers under its own wallet (a wallet holds at
//! most one address) with a process-unique, globally-unique username. Nostr is
//! left off (`allows_nostr: false`) — its serving path is mocked separately.

use reqwest::StatusCode;

use swissknife_types::{LnAddress, RegisterLnAddressRequest, UpdateLnAddressRequest};

use crate::common::fixtures::unique;
use crate::common::{app, assert_error, assert_status, Auth};

fn register_req(wallet_id: uuid::Uuid, username: &str) -> RegisterLnAddressRequest {
    RegisterLnAddressRequest {
        wallet_id: Some(wallet_id),
        username: username.to_string(),
        allows_nostr: false,
        nostr_pubkey: None,
    }
}

mod register {
    use super::*;

    #[tokio::test]
    async fn creates_an_active_address() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnaddr-create").await;
        let username = unique("lnaddr");

        let res = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(token),
                register_req(wallet.id, &username),
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let addr = res.parse::<LnAddress>();
        assert_eq!(addr.username, username);
        assert_eq!(addr.wallet_id, wallet.id);
        assert!(addr.active, "a newly registered address is active");
        assert!(!addr.allows_nostr);
    }

    #[tokio::test]
    async fn requires_authentication() {
        let app = app().await;
        let res = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::None,
                register_req(uuid::Uuid::new_v4(), "noauth"),
            )
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_an_invalid_username() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnaddr-badname").await;

        // Uppercase and a space violate the username format (422).
        let res = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(token),
                register_req(wallet.id, "Invalid Name"),
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn rejects_a_duplicate_username() {
        let app = app().await;
        let token = app.admin_token().await;
        let first = app.create_wallet(token, "lnaddr-dup-a").await;
        let second = app.create_wallet(token, "lnaddr-dup-b").await;
        let username = unique("lnaddr-dup");

        let res = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(token),
                register_req(first.id, &username),
            )
            .await;
        assert_status(&res, StatusCode::OK);

        // The same username on another wallet conflicts.
        let dup = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(token),
                register_req(second.id, &username),
            )
            .await;
        assert_error(&dup, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn rejects_a_second_address_for_one_wallet() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnaddr-one").await;

        let res = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(token),
                register_req(wallet.id, &unique("lnaddr-one")),
            )
            .await;
        assert_status(&res, StatusCode::OK);

        // A wallet may hold only one address.
        let again = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(token),
                register_req(wallet.id, &unique("lnaddr-two")),
            )
            .await;
        assert_error(&again, StatusCode::CONFLICT);
    }
}

mod manage {
    use super::*;

    #[tokio::test]
    async fn get_update_then_delete() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "lnaddr-manage").await;
        let username = unique("lnaddr-mng");

        let created = app
            .api()
            .post(
                "/v1/lightning-addresses",
                Auth::Bearer(token),
                register_req(wallet.id, &username),
            )
            .await
            .parse::<LnAddress>();

        let got = app
            .api()
            .get(&format!("/v1/lightning-addresses/{}", created.id), Auth::Bearer(token))
            .await;
        assert_status(&got, StatusCode::OK);
        assert_eq!(got.parse::<LnAddress>().username, username);

        let list = app
            .api()
            .get(
                &format!("/v1/lightning-addresses?wallet_id={}", wallet.id),
                Auth::Bearer(token),
            )
            .await;
        assert_status(&list, StatusCode::OK);
        assert_eq!(
            list.parse::<Vec<LnAddress>>().len(),
            1,
            "the wallet has exactly one address"
        );

        // Deactivate via update.
        let updated = app
            .api()
            .put(
                &format!("/v1/lightning-addresses/{}", created.id),
                Auth::Bearer(token),
                UpdateLnAddressRequest {
                    username: None,
                    active: Some(false),
                    allows_nostr: None,
                    nostr_pubkey: None,
                },
            )
            .await;
        assert_status(&updated, StatusCode::OK);
        assert!(!updated.parse::<LnAddress>().active, "the address is deactivated");

        let del = app
            .api()
            .delete(&format!("/v1/lightning-addresses/{}", created.id), Auth::Bearer(token))
            .await;
        assert_status(&del, StatusCode::OK);

        let gone = app
            .api()
            .get(&format!("/v1/lightning-addresses/{}", created.id), Auth::Bearer(token))
            .await;
        assert_error(&gone, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_id_is_not_found() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app
            .api()
            .get(
                &format!("/v1/lightning-addresses/{}", uuid::Uuid::new_v4()),
                Auth::Bearer(token),
            )
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }
}

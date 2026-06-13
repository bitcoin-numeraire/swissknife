//! `/v1/bitcoin/addresses` — on-chain deposit addresses, permission-gated
//! (`*:btc_address`). Covers the address types that matter in practice, P2WPKH
//! and P2TR, both supported on every Lightning backend.

use std::time::Duration;

use reqwest::StatusCode;

use swissknife_types::{BtcAddress, BtcAddressType, NewBtcAddressRequest};

use crate::common::chain::{mine, send_to_address};
use crate::common::client::TestResponse;
use crate::common::wait::wait_until;
use crate::common::{app, assert_error, assert_status, Auth, TestApp};

async fn post_address(
    app: &TestApp,
    token: &str,
    wallet_id: Option<uuid::Uuid>,
    address_type: Option<BtcAddressType>,
) -> TestResponse {
    app.api()
        .post(
            "/v1/bitcoin/addresses",
            Auth::Bearer(token),
            NewBtcAddressRequest {
                wallet_id,
                address_type,
            },
        )
        .await
}

mod generate {
    use super::*;

    #[tokio::test]
    async fn p2wpkh_and_p2tr_work_on_every_backend() {
        let app = app().await;
        let token = app.admin_token().await;

        for (ty, prefix) in [(BtcAddressType::P2wpkh, "bcrt1q"), (BtcAddressType::P2tr, "bcrt1p")] {
            let res = post_address(app, token, None, Some(ty)).await;
            assert_status(&res, StatusCode::OK);
            let addr = res.parse::<BtcAddress>();
            assert_eq!(addr.address_type, ty);
            assert!(!addr.used, "a fresh address is unused");
            assert!(
                addr.address.starts_with(prefix),
                "{ty:?} address {} should start with {prefix}",
                addr.address
            );
        }
    }

    #[tokio::test]
    async fn defaults_to_p2wpkh_when_type_is_omitted() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = post_address(app, token, None, None).await;
        assert_status(&res, StatusCode::OK);
        assert_eq!(res.parse::<BtcAddress>().address_type, BtcAddressType::P2wpkh);
    }

    #[tokio::test]
    async fn reuses_the_unused_deposit_address_per_type() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "btc-reuse").await;
        let first = post_address(app, token, Some(wallet.id), Some(BtcAddressType::P2wpkh))
            .await
            .parse::<BtcAddress>();
        let second = post_address(app, token, Some(wallet.id), Some(BtcAddressType::P2wpkh))
            .await
            .parse::<BtcAddress>();
        assert_eq!(
            first.id, second.id,
            "an unused deposit address is reused, not regenerated"
        );
        assert_eq!(first.address, second.address);
    }

    #[tokio::test]
    async fn requires_authentication() {
        let app = app().await;
        let res = app
            .api()
            .post(
                "/v1/bitcoin/addresses",
                Auth::None,
                NewBtcAddressRequest {
                    wallet_id: None,
                    address_type: None,
                },
            )
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }
}

mod query {
    use super::*;

    #[tokio::test]
    async fn get_by_id_and_filter_by_wallet_and_type() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "btc-query").await;

        let created = post_address(app, token, Some(wallet.id), Some(BtcAddressType::P2tr)).await;
        assert_status(&created, StatusCode::OK);
        let created = created.parse::<BtcAddress>();

        let got = app
            .api()
            .get(&format!("/v1/bitcoin/addresses/{}", created.id), Auth::Bearer(token))
            .await;
        assert_status(&got, StatusCode::OK);
        assert_eq!(got.parse::<BtcAddress>().address, created.address);

        let list = app
            .api()
            .get(
                &format!("/v1/bitcoin/addresses?wallet_id={}&address_type=p2tr", wallet.id),
                Auth::Bearer(token),
            )
            .await;
        assert_status(&list, StatusCode::OK);
        let addresses = list.parse::<Vec<BtcAddress>>();
        assert!(
            addresses.iter().any(|a| a.id == created.id),
            "created address is listed"
        );
        assert!(
            addresses
                .iter()
                .all(|a| a.address_type == BtcAddressType::P2tr && a.wallet_id == wallet.id),
            "filter restricts to the requested wallet and type"
        );
    }

    #[tokio::test]
    async fn list_respects_the_limit() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "btc-limit").await;

        // Distinct types are distinct rows (an unused address is reused per type).
        assert_status(
            &post_address(app, token, Some(wallet.id), Some(BtcAddressType::P2wpkh)).await,
            StatusCode::OK,
        );
        assert_status(
            &post_address(app, token, Some(wallet.id), Some(BtcAddressType::P2tr)).await,
            StatusCode::OK,
        );

        let all = app
            .api()
            .get(
                &format!("/v1/bitcoin/addresses?wallet_id={}", wallet.id),
                Auth::Bearer(token),
            )
            .await;
        assert!(
            all.parse::<Vec<BtcAddress>>().len() >= 2,
            "two distinct types are two rows"
        );

        let limited = app
            .api()
            .get(
                &format!("/v1/bitcoin/addresses?wallet_id={}&limit=1", wallet.id),
                Auth::Bearer(token),
            )
            .await;
        assert_status(&limited, StatusCode::OK);
        assert_eq!(limited.parse::<Vec<BtcAddress>>().len(), 1, "limit caps the page");
    }

    #[tokio::test]
    async fn unknown_id_is_not_found() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app
            .api()
            .get(
                &format!("/v1/bitcoin/addresses/{}", uuid::Uuid::new_v4()),
                Auth::Bearer(token),
            )
            .await;
        assert_error(&res, StatusCode::NOT_FOUND);
    }
}

mod deposit {
    use super::*;

    #[tokio::test]
    async fn a_taproot_deposit_credits_the_wallet_and_marks_the_address_used() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "btc-deposit").await;

        let res = post_address(app, token, Some(wallet.id), Some(BtcAddressType::P2tr)).await;
        assert_status(&res, StatusCode::OK);
        let addr = res.parse::<BtcAddress>();
        assert!(!addr.used);

        let sats = 500_000u64;
        send_to_address(&addr.address, sats).await;
        mine(6).await;

        let target = sats as i64 * 1000;
        wait_until(Duration::from_secs(90), "taproot deposit credited", || async {
            app.wallet_balance(token, wallet.id).await.available_msat >= target
        })
        .await;

        // The deposit projection marks the receiving address used in the same
        // transaction that credits the balance.
        let list = app
            .api()
            .get(
                &format!("/v1/bitcoin/addresses?wallet_id={}&used=true", wallet.id),
                Auth::Bearer(token),
            )
            .await;
        assert_status(&list, StatusCode::OK);
        assert!(
            list.parse::<Vec<BtcAddress>>().iter().any(|a| a.id == addr.id),
            "the funded address is reported as used"
        );
    }
}

mod delete {
    use super::*;

    #[tokio::test]
    async fn deletes_a_single_unused_address() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "btc-del-one").await;
        let addr = post_address(app, token, Some(wallet.id), None)
            .await
            .parse::<BtcAddress>();

        let del = app
            .api()
            .delete(&format!("/v1/bitcoin/addresses/{}", addr.id), Auth::Bearer(token))
            .await;
        assert_status(&del, StatusCode::OK);

        let got = app
            .api()
            .get(&format!("/v1/bitcoin/addresses/{}", addr.id), Auth::Bearer(token))
            .await;
        assert_error(&got, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn bulk_deletes_by_wallet_filter() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "btc-del-bulk").await;
        assert_status(
            &post_address(app, token, Some(wallet.id), Some(BtcAddressType::P2wpkh)).await,
            StatusCode::OK,
        );
        assert_status(
            &post_address(app, token, Some(wallet.id), Some(BtcAddressType::P2tr)).await,
            StatusCode::OK,
        );

        let before = app
            .api()
            .get(
                &format!("/v1/bitcoin/addresses?wallet_id={}", wallet.id),
                Auth::Bearer(token),
            )
            .await
            .parse::<Vec<BtcAddress>>()
            .len() as u64;

        let del = app
            .api()
            .delete(
                &format!("/v1/bitcoin/addresses?wallet_id={}", wallet.id),
                Auth::Bearer(token),
            )
            .await;
        assert_status(&del, StatusCode::OK);
        assert_eq!(del.parse::<u64>(), before, "bulk delete removes every matching address");

        let after = app
            .api()
            .get(
                &format!("/v1/bitcoin/addresses?wallet_id={}", wallet.id),
                Auth::Bearer(token),
            )
            .await;
        assert_eq!(after.parse::<Vec<BtcAddress>>().len(), 0);
    }
}

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
    wallet_id: uuid::Uuid,
    address_type: Option<BtcAddressType>,
) -> TestResponse {
    app.api()
        .post(
            "/v1/bitcoin/addresses",
            Auth::Bearer(token),
            NewBtcAddressRequest {
                wallet_id: Some(wallet_id),
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
        let wallet = app.create_wallet(token, "btc-types").await;

        for (ty, prefix) in [(BtcAddressType::P2wpkh, "bcrt1q"), (BtcAddressType::P2tr, "bcrt1p")] {
            let res = post_address(app, token, wallet.id, Some(ty)).await;
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
        let wallet = app.create_wallet(token, "btc-default-type").await;
        let res = post_address(app, token, wallet.id, None).await;
        assert_status(&res, StatusCode::OK);
        assert_eq!(res.parse::<BtcAddress>().address_type, BtcAddressType::P2wpkh);
    }

    #[tokio::test]
    async fn reuses_the_unused_deposit_address_per_type() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "btc-reuse").await;
        let first = post_address(app, token, wallet.id, Some(BtcAddressType::P2wpkh))
            .await
            .parse::<BtcAddress>();
        let second = post_address(app, token, wallet.id, Some(BtcAddressType::P2wpkh))
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
                    wallet_id: Some(uuid::Uuid::new_v4()),
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

        let created = post_address(app, token, wallet.id, Some(BtcAddressType::P2tr)).await;
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
            &post_address(app, token, wallet.id, Some(BtcAddressType::P2wpkh)).await,
            StatusCode::OK,
        );
        assert_status(
            &post_address(app, token, wallet.id, Some(BtcAddressType::P2tr)).await,
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

        let res = post_address(app, token, wallet.id, Some(BtcAddressType::P2tr)).await;
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

mod withdraw {
    use super::*;

    use crate::common::chain::{new_address, received_by_address};
    use swissknife_types::{Ledger, Payment, PaymentStatus, SendPaymentRequest};

    /// Paying an external (non-SwissKnife) address is a real broadcast, not an
    /// internal settlement: the payment is created Pending, the reservation
    /// debits the wallet, and the on-chain listener settles it on confirmation.
    #[tokio::test]
    async fn broadcasts_on_chain_and_settles_on_confirmation() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "btc-withdraw").await;

        // Fund the wallet on-chain so the node holds a spendable UTXO.
        app.fund_onchain(token, wallet.id, 1_000_000).await;
        let before = app.wallet_balance(token, wallet.id).await.available_msat;

        let target = new_address().await;
        let amount_msat = 200_000_000u64; // 200k sat

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                SendPaymentRequest {
                    wallet_id: Some(wallet.id),
                    input: target.clone(),
                    amount_msat: Some(amount_msat),
                    comment: None,
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let payment = res.parse::<Payment>();
        assert_eq!(
            payment.ledger,
            Ledger::Onchain,
            "an external btc address is an on-chain send"
        );
        assert_eq!(payment.status, PaymentStatus::Pending, "broadcast, not yet confirmed");
        let btc = payment
            .bitcoin
            .as_ref()
            .expect("an on-chain payment carries bitcoin details");
        assert_eq!(btc.address, target);
        assert!(!btc.txid.is_empty(), "a broadcast transaction has a txid");

        // The reservation debits the wallet immediately (amount + on-chain fee).
        let reserved = app.wallet_balance(token, wallet.id).await.available_msat;
        assert!(
            reserved <= before - amount_msat as i64,
            "the wallet is debited by at least the amount (before={before}, after={reserved})"
        );

        // Confirm the broadcast; the on-chain listener settles the payment.
        mine(6).await;
        wait_until(
            Duration::from_secs(90),
            "withdrawal settles on confirmation",
            || async {
                let res = app
                    .api()
                    .get(&format!("/v1/payments/{}", payment.id), Auth::Bearer(token))
                    .await;
                res.status.as_u16() == 200 && res.parse::<Payment>().status == PaymentStatus::Settled
            },
        )
        .await;

        // The sats actually landed at the external address on-chain.
        assert!(
            received_by_address(&target).await >= 200_000,
            "the withdrawal was broadcast and confirmed on-chain"
        );
    }

    /// Paying a deposit address owned by another SwissKnife wallet settles
    /// internally — synchronous, no broadcast — and credits the recipient.
    #[tokio::test]
    async fn settles_internally_to_a_swissknife_address() {
        let app = app().await;
        let token = app.admin_token().await;
        let payer = app.create_wallet(token, "btc-int-payer").await;
        let payee = app.create_wallet(token, "btc-int-payee").await;
        app.fund_onchain(token, payer.id, 1_000_000).await;

        let payee_addr = post_address(app, token, payee.id, None)
            .await
            .parse::<BtcAddress>()
            .address;

        let amount_msat = 300_000_000u64;
        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                SendPaymentRequest {
                    wallet_id: Some(payer.id),
                    input: payee_addr,
                    amount_msat: Some(amount_msat),
                    comment: None,
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        let payment = res.parse::<Payment>();
        assert_eq!(
            payment.ledger,
            Ledger::Internal,
            "paying a SwissKnife-owned address settles internally"
        );
        assert_eq!(
            payment.status,
            PaymentStatus::Settled,
            "internal settlement is synchronous"
        );
        assert!(
            app.wallet_balance(token, payee.id).await.available_msat >= amount_msat as i64,
            "the internal recipient is credited"
        );
    }

    /// The ledger reservation guards the balance: an on-chain send beyond it is
    /// rejected (422) before anything is broadcast. Fund the shared node wallet
    /// through one wallet so the transaction can be built, then overdraw a
    /// different, unfunded wallet — isolating the ledger reserve as the rejecter
    /// rather than the node failing to fund the tx (which differs by backend).
    #[tokio::test]
    async fn rejects_a_withdrawal_beyond_the_balance() {
        let app = app().await;
        let token = app.admin_token().await;
        let funded = app.create_wallet(token, "btc-od-funded").await;
        app.fund_onchain(token, funded.id, 1_000_000).await;
        let broke = app.create_wallet(token, "btc-od-broke").await;

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                SendPaymentRequest {
                    wallet_id: Some(broke.id),
                    input: new_address().await,
                    amount_msat: Some(500_000_000),
                    comment: None,
                },
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// Paying your own deposit address is rejected.
    #[tokio::test]
    async fn rejects_paying_your_own_address() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "btc-self").await;
        let own = post_address(app, token, wallet.id, None)
            .await
            .parse::<BtcAddress>()
            .address;

        let res = app
            .api()
            .post(
                "/v1/payments",
                Auth::Bearer(token),
                SendPaymentRequest {
                    wallet_id: Some(wallet.id),
                    input: own,
                    amount_msat: Some(100_000_000),
                    comment: None,
                },
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

mod delete {
    use super::*;

    #[tokio::test]
    async fn deletes_a_single_unused_address() {
        let app = app().await;
        let token = app.admin_token().await;
        let wallet = app.create_wallet(token, "btc-del-one").await;
        let addr = post_address(app, token, wallet.id, None).await.parse::<BtcAddress>();

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
            &post_address(app, token, wallet.id, Some(BtcAddressType::P2wpkh)).await,
            StatusCode::OK,
        );
        assert_status(
            &post_address(app, token, wallet.id, Some(BtcAddressType::P2tr)).await,
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

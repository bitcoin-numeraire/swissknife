//! Opt-in interoperability with public signet/testnet services. This suite
//! creates an ephemeral, unfunded wallet; it does not establish paid Lightning
//! routing, boarding confirmation, fee accounting or funded recovery coverage.
#![cfg(feature = "public-network-tests")]

use std::{path::PathBuf, time::Duration};
use wavelength_client::{rpc, Client, Config};

// Only an ephemeral, unfunded test wallet uses this password. Never import a
// funded seed into this suite: its daemon and all local state are discarded.
const PASSWORD: &[u8] = b"swissknife-public-network-test";

async fn ready(client: &Client, network: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    loop {
        match client.status(Default::default()).await {
            Ok(status) if status.ready => {
                assert!(status.unlocked);
                assert_eq!(status.network, network);
                return;
            }
            result if tokio::time::Instant::now() >= deadline => {
                panic!("public-network wallet did not become ready: {result:?}");
            }
            _ => tokio::time::sleep(Duration::from_secs(2)).await,
        }
    }
}

#[tokio::test]
async fn unfunded_wallet_uses_public_services_and_preserves_identity_and_activity_after_restart() {
    let network = std::env::var("WAVELENGTH_PUBLIC_NETWORK").expect("public test network");
    assert!(matches!(network.as_str(), "signet" | "testnet"), "test networks only");
    let dir = PathBuf::from(std::env::var("WAVELENGTH_TEST_CREDENTIAL_DIR").expect("credential directory"));
    let client = Client::connect(Config {
        endpoint: std::env::var("WAVELENGTH_TEST_ENDPOINT").expect("daemon endpoint"),
        certificate_pem: std::fs::read(dir.join("tls.cert")).expect("daemon certificate"),
        macaroon: std::fs::read(dir.join("admin.macaroon")).expect("daemon macaroon"),
        connect_timeout: Duration::from_secs(10),
        request_timeout: Duration::from_secs(60),
    })
    .await
    .expect("verified daemon connection");

    let phase = std::env::var("WAVELENGTH_TEST_PHASE").expect("create or restart phase");
    let identity;
    let receive_id;
    match phase.as_str() {
        "create" => {
            let response = client
                .create(rpc::CreateRequest {
                    wallet_password: PASSWORD.to_vec(),
                    ..Default::default()
                })
                .await
                .expect("create ephemeral wallet through the public API");
            // Never print, persist or include the generated seed in assertions.
            assert_eq!(response.mnemonic.len(), 24);
            assert_eq!(response.identity_pubkey.len(), 66);
            identity = response.identity_pubkey;
            drop(response.mnemonic);
            ready(&client, &network).await;
            assert!(client.activity(10, 10).await.expect("initial activity").is_empty());
            let deposit = client
                .deposit(Default::default())
                .await
                .expect("public boarding address");
            assert!(deposit.onchain_address.starts_with("tb1"));
            let deposit_entry = deposit.entry.expect("boarding request summary");
            assert_eq!(deposit_entry.kind, rpc::EntryKind::Deposit as i32);
            // Merely allocating an address does not create a payment or funds
            // in flight. Only an observed UTXO becomes deposit activity.
            assert!(client
                .activity(10, 10)
                .await
                .expect("unfunded deposit activity")
                .is_empty());
            let receive = client
                .recv(rpc::RecvRequest {
                    amt_sat: 21_000,
                    memo: "SwissKnife public-network interoperability test".into(),
                })
                .await
                .expect("unpaid receive invoice through the public swap service");
            assert!(receive
                .invoice
                .starts_with(if network == "signet" { "lntbs" } else { "lntb" }));
            receive_id = receive.entry.expect("receive activity").id;
            assert!(!receive_id.is_empty());
            std::fs::write(dir.join("expected-identity"), &identity).expect("save public identity");
            std::fs::write(dir.join("expected-receive"), &receive_id).expect("save activity identifier");
        }
        "restart" => {
            // Unlock once; never retry a wallet-initialization mutation on error.
            identity = client
                .unlock(rpc::UnlockRequest {
                    wallet_password: PASSWORD.to_vec(),
                })
                .await
                .expect("unlock existing wallet after daemon restart")
                .identity_pubkey;
            assert_eq!(
                identity,
                std::fs::read_to_string(dir.join("expected-identity")).expect("original identity")
            );
            receive_id = std::fs::read_to_string(dir.join("expected-receive")).expect("original activity identifier");
            ready(&client, &network).await;
        }
        _ => panic!("expected create or restart phase"),
    }
    let balance = client.balance(Default::default()).await.expect("public wallet balance");
    assert_eq!(balance.confirmed_sat, 0);
    assert_eq!(balance.pending_out_sat, 0);
    assert_eq!(balance.credit_available_sat, 0);
    assert_eq!(balance.credit_reserved_sat, 0);
    let activity = client.activity(1, 10).await.expect("complete durable activity");
    assert_eq!(activity.len(), 1);
    assert_eq!(activity[0].id, receive_id);
    assert_eq!(activity[0].kind, rpc::EntryKind::Recv as i32);
    assert_eq!(activity[0].status, rpc::EntryStatus::Pending as i32);
    assert_eq!(activity[0].amount_sat, 21_000);
}

//! Real, wallet-enabled waved in Docker. These tests deliberately exercise the
//! locked-daemon transport contract; funded Ark/swap lifecycle coverage belongs
//! to the provider integration gate and is not claimed by this suite.
#![cfg(feature = "daemon-tests")]

use std::{path::PathBuf, time::Duration};
use wavelength_client::{rpc, Client, Config, Error};

fn config() -> Config {
    let dir = PathBuf::from(std::env::var("WAVELENGTH_TEST_CREDENTIAL_DIR").expect("daemon credential directory"));
    Config {
        endpoint: std::env::var("WAVELENGTH_TEST_ENDPOINT").expect("daemon endpoint"),
        certificate_pem: std::fs::read(dir.join("tls.cert")).expect("daemon certificate"),
        macaroon: std::fs::read(dir.join("admin.macaroon")).expect("daemon macaroon"),
        connect_timeout: Duration::from_secs(5),
        request_timeout: Duration::from_secs(5),
    }
}

#[tokio::test]
async fn distinguishes_an_authenticated_uninitialized_wallet_from_bad_credentials() {
    let client = Client::connect(config()).await.expect("verified daemon connection");
    // The pinned daemon calls Balance from Status, so Status returns
    // FailedPrecondition before Create rather than a ready=false response.
    assert!(
        matches!(client.status(Default::default()).await, Err(Error::Rpc(status)) if status.code() == tonic::Code::FailedPrecondition)
    );
    assert!(client
        .recv(rpc::RecvRequest {
            amt_sat: 21000,
            memo: "locked wallet".into()
        })
        .await
        .is_err());
}

#[tokio::test]
async fn daemon_rejects_an_invalid_macaroon() {
    let mut config = config();
    config.macaroon = b"invalid macaroon".to_vec();
    let client = Client::connect(config).await.expect("TLS connection");
    assert!(
        matches!(client.status(Default::default()).await, Err(Error::Rpc(status)) if status.code() == tonic::Code::Unauthenticated)
    );
}

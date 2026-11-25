use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use bitcoin::hashes::{sha256, Hash};
use l402_proto::{L402Challenge, L402Token};
use std::sync::atomic::{AtomicUsize, Ordering};
use tower::ServiceExt;

const PREIMAGE: [u8; 32] = [49; 32];

struct AcceptOnce(AtomicUsize);

#[async_trait]
impl InvoiceGate for AcceptOnce {
    async fn invoice(&self) -> Result<ChallengeInvoice, GateError> {
        Ok(ChallengeInvoice {
            payment_hash: sha256::Hash::hash(&PREIMAGE).to_byte_array(),
            bolt11: "lnbcrt1fixture".into(),
        })
    }
    async fn redeem(&self, _grant: &PaidGrant) -> Result<bool, GateError> {
        Ok(self.0.fetch_add(1, Ordering::SeqCst) == 0)
    }
}

#[tokio::test]
async fn signed_challenge_gates_the_normal_axum_route_and_strips_payment_credentials() {
    let gate = Arc::new(PaidRoute {
        scope: "wallet/catalog/v1".into(),
        validity: Duration::from_secs(60),
        bakery: Bakery::new(&[7; 32]),
        gate: Arc::new(AcceptOnce(AtomicUsize::new(0))),
    });
    let route = Router::new()
        .route(
            "/catalog",
            get(|request: Request<Body>| async move {
                assert!(request.headers().get(header::AUTHORIZATION).is_none());
                assert!(request.extensions().get::<PaidGrant>().is_some());
                "paid content"
            }),
        )
        .route_layer(middleware::from_fn_with_state(gate, require_payment));
    let request = |auth: Option<&str>| {
        let mut r = Request::builder().uri("/catalog");
        if let Some(auth) = auth {
            r = r.header(header::AUTHORIZATION, auth);
        }
        r.body(Body::empty()).unwrap()
    };
    let first = route.clone().oneshot(request(None)).await.unwrap();
    assert_eq!(first.status(), StatusCode::PAYMENT_REQUIRED);
    let challenge = L402Challenge::from_header(first.headers()[header::WWW_AUTHENTICATE].to_str().unwrap()).unwrap();
    let token = L402Token::new(challenge.macaroon, hex::encode(PREIMAGE)).to_header_value();
    let accepted = route.clone().oneshot(request(Some(&token))).await.unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
    assert_eq!(accepted.headers()[header::CACHE_CONTROL], "private, no-store");
    assert_eq!(
        route.oneshot(request(Some(&token))).await.unwrap().status(),
        StatusCode::PAYMENT_REQUIRED
    );
}

#[test]
fn scope_expiry_signature_and_preimage_are_independently_required() {
    let bakery = Bakery::new(&[7; 32]);
    let payment_hash = sha256::Hash::hash(&PREIMAGE).to_byte_array();
    let encoded = bakery.mint("published/report", payment_hash, 105).unwrap();
    let token = L402Token::new(encoded.clone(), hex::encode(PREIMAGE)).to_header_value();
    let proof = bakery.verify("published/report", &token, 104).unwrap();
    assert_eq!(proof.payment_hash, payment_hash);
    assert!(bakery.verify("another/report", &token, 104).is_err());
    assert!(bakery.verify("published/report", &token, 105).is_err());
    assert!(Bakery::new(&[8; 32]).verify("published/report", &token, 104).is_err());
    assert!(bakery
        .verify(
            "published/report",
            &L402Token::new(encoded.clone(), hex::encode([0; 32])).to_header_value(),
            104
        )
        .is_err());
    let mut shorter = libmacaroon::Macaroon::deserialize(encoded.as_bytes()).unwrap();
    shorter.add_first_party_caveat("expires=103").unwrap();
    let token = L402Token::new(
        shorter.serialize(libmacaroon::Format::V2).unwrap(),
        hex::encode(PREIMAGE),
    )
    .to_header_value();
    assert!(bakery.verify("published/report", &token, 102).is_ok());
    assert!(bakery.verify("published/report", &token, 103).is_err());
}

// Marshaled independently with Go gopkg.in/macaroon.v2 v2.1.0, using
// Aperture's version-zero identifier layout and the raw 32-byte root key.
#[test]
fn accepts_independently_signed_go_v2_macaroon() {
    let encoded = "AgJCAACKg2ZfN5hyfxT5KtDmyZ/asI7nMdbNZEwTEiP9L0/tKgkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJAAIWc2NvcGU9cHVibGlzaGVkL3JlcG9ydAACC2V4cGlyZXM9MTA1AAAGIIK5OxnVFom/LkERtkYeno1ZoKDiMbM5Gaez89fVBtOV";
    let header = L402Token::new(encoded.to_owned(), hex::encode(PREIMAGE)).to_header_value();
    let grant = Bakery::new(&[7; 32]).verify("published/report", &header, 104).unwrap();
    assert_eq!(grant.payment_hash, sha256::Hash::hash(&PREIMAGE).to_byte_array());
    assert_eq!(grant.token_id, [9; 32]);
}

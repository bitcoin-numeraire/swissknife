use std::{sync::Arc, vec};

use async_trait::async_trait;
use bitcoin::hashes::{sha256, Hash};
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    application::{
        composition::AppStore,
        errors::{ApplicationError, DataError, LightningError},
    },
    domains::invoice::{InvoiceFilter, InvoiceStatus, LnInvoiceOptions, LnUrlInvoiceData},
    infra::lightning::LnClient,
};
use swissknife_types::LnUrlVerifyResponse;

use super::{LnURLPayRequest, LnUrlCallback, LnUrlSuccessAction, LnUrlUseCases};

const MIN_SENDABLE: u64 = 1000;
const MAX_SENDABLE: u64 = 250000000;
const COMMENT_ALLOWED: u16 = 255;

pub struct LnUrlService {
    domain: String,
    host: String,
    store: AppStore,
    invoice_expiry: u32,
    ln_client: Arc<dyn LnClient>,
}

impl LnUrlService {
    pub fn new(
        store: AppStore,
        ln_client: Arc<dyn LnClient>,
        invoice_expiry: u32,
        domain: String,
        host: String,
    ) -> Self {
        LnUrlService {
            store,
            ln_client,
            invoice_expiry,
            domain,
            host,
        }
    }

    fn metadata(&self, username: &str) -> String {
        serde_json::to_string(&[
            ["text/identifier".to_string(), format!("{}@{}", username, self.domain)],
            ["text/plain".to_string(), format!("{} never refuses sats", username)],
        ])
        .expect("should not fail as a constant")
    }
}

#[async_trait]
impl LnUrlUseCases for LnUrlService {
    async fn lnurlp(&self, username: String) -> Result<LnURLPayRequest, ApplicationError> {
        debug!(username, "Generating LNURLp");

        let ln_address = self
            .store
            .ln_address
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        if !ln_address.active {
            return Err(DataError::NotFound("Lightning address not found.".to_string()).into());
        }

        let lnurlp = LnURLPayRequest {
            callback: format!("{}/lnurlp/{}/callback", self.host, username),
            max_sendable: MAX_SENDABLE,
            min_sendable: MIN_SENDABLE,
            metadata: self.metadata(&username),
            comment_allowed: COMMENT_ALLOWED,
            tag: "payRequest".to_string(),
            allows_nostr: ln_address.allows_nostr,
            nostr_pubkey: ln_address.nostr_pubkey,
        };

        info!(username, "LNURLp returned successfully");
        Ok(lnurlp)
    }

    async fn lnurlp_callback(
        &self,
        username: String,
        amount: u64,
        comment: Option<String>,
    ) -> Result<LnUrlCallback, ApplicationError> {
        if !(MIN_SENDABLE..=MAX_SENDABLE).contains(&amount) {
            return Err(DataError::Validation(format!(
                "Amount must be between {MIN_SENDABLE} and {MAX_SENDABLE} millisatoshis"
            ))
            .into());
        }

        debug!(username, amount, comment, "Generating LNURLp invoice");

        let ln_address = self
            .store
            .ln_address
            .find_by_username(&username)
            .await?
            .ok_or_else(|| DataError::NotFound("Lightning address not found.".to_string()))?;

        if !ln_address.active {
            return Err(DataError::NotFound("Lightning address not found.".to_string()).into());
        }

        let invoice_id = Uuid::new_v4();
        let verify_token = Uuid::new_v4();
        // SwissKnife must know the preimage even if the invoice is paid internally
        // and the provider's copy is canceled.
        let preimage: [u8; 32] = rand::random();
        let mut invoice = self
            .ln_client
            .invoice(
                amount,
                self.metadata(&username),
                invoice_id.to_string(),
                self.invoice_expiry,
                LnInvoiceOptions {
                    description_hash_only: true,
                    preimage: Some(preimage),
                },
            )
            .await?;
        let ln_invoice = invoice
            .ln_invoice
            .as_ref()
            .ok_or_else(|| LightningError::Invoice("Provider returned no Lightning invoice".to_string()))?;
        if ln_invoice.payment_hash != sha256::Hash::hash(&preimage).to_string() {
            return Err(LightningError::Invoice("Provider returned an unexpected invoice payment hash".into()).into());
        }
        let bolt11 = ln_invoice.bolt11.clone();
        invoice.id = invoice_id;
        invoice.wallet_id.clone_from(&ln_address.wallet_id);
        invoice.ln_address_id = Some(ln_address.id);
        invoice.description = Some(comment.unwrap_or(format!("Payment to {}@{}", username, self.domain)));
        invoice.lnurl = Some(LnUrlInvoiceData {
            verify_token,
            preimage: hex::encode(preimage),
        });

        // TODO: Get or add more information to make this a LNURLp invoice (like fetching a success action specific to the user)
        self.store.invoice.insert(invoice).await?;
        let lnurlp_invoice = LnUrlCallback {
            pr: bolt11,
            success_action: Some(LnUrlSuccessAction {
                tag: "message".to_string(),
                message: Some("Thanks for the sats!".to_string()),
                ..Default::default()
            }),
            disposable: None,
            routes: vec![],
            verify: Some(format!(
                "{}/lnurlp/{verify_token}/verify",
                self.host.trim_end_matches('/')
            )),
        };

        info!(username, "Lightning invoice generated successfully");
        Ok(lnurlp_invoice)
    }

    async fn verify(&self, token: Uuid) -> Result<LnUrlVerifyResponse, ApplicationError> {
        let invoice = self
            .store
            .invoice
            .find_many(InvoiceFilter {
                lnurl_verify_token: Some(token),
                limit: Some(1),
                ..Default::default()
            })
            .await?
            .into_iter()
            .next();
        let Some(invoice) = invoice else {
            return Ok(LnUrlVerifyResponse::Error {
                reason: "Not found".into(),
            });
        };
        let lnurl = invoice
            .lnurl
            .ok_or_else(|| DataError::Inconsistency("Missing LNURL verification data".into()))?;
        let ln_invoice = invoice
            .ln_invoice
            .ok_or_else(|| DataError::Inconsistency("Missing Lightning invoice details".into()))?;
        let settled = invoice.status == InvoiceStatus::Settled;
        Ok(LnUrlVerifyResponse::Ok {
            settled,
            preimage: settled.then_some(lnurl.preimage),
            pr: ln_invoice.bolt11,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        application::composition::MockAppStoreBuilder,
        domains::{
            invoice::{Invoice, LnInvoice},
            ln_address::LnAddress,
        },
        infra::lightning::MockLnClient,
    };

    use super::*;

    fn service(store: MockAppStoreBuilder, ln_client: MockLnClient) -> LnUrlService {
        LnUrlService::new(
            store.build(),
            Arc::new(ln_client),
            3_600,
            "numeraire.tech".to_string(),
            "https://numeraire.tech".to_string(),
        )
    }

    fn ln_address(active: bool) -> LnAddress {
        LnAddress {
            id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            wallet_id: Uuid::new_v4(),
            username: "alice".to_string(),
            active,
            allows_nostr: false,
            nostr_pubkey: None,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    mod verify {
        use crate::application::errors::DatabaseError;

        use super::*;

        fn verification_invoice(status: InvoiceStatus) -> Invoice {
            Invoice {
                status,
                ln_invoice: Some(LnInvoice {
                    bolt11: "original-bolt11".into(),
                    ..Default::default()
                }),
                lnurl: Some(LnUrlInvoiceData {
                    verify_token: Uuid::new_v4(),
                    preimage: "11".repeat(32),
                }),
                ..Default::default()
            }
        }

        fn with_invoice(mut invoice: Invoice) -> (LnUrlService, Uuid) {
            let token = Uuid::new_v4();
            invoice.lnurl.as_mut().unwrap().verify_token = token;
            let mut store = MockAppStoreBuilder::new();
            store
                .invoice
                .expect_find_many()
                .withf(move |filter| filter.lnurl_verify_token == Some(token) && filter.limit == Some(1))
                .times(1)
                .return_once(move |_| Ok(vec![invoice]));
            (service(store, MockLnClient::new()), token)
        }

        mod when_unsettled {
            use super::*;

            #[tokio::test]
            async fn withholds_the_preimage_for_pending_and_expired_invoices() {
                for status in [InvoiceStatus::Pending, InvoiceStatus::Expired] {
                    let (service, token) = with_invoice(verification_invoice(status));
                    assert!(matches!(
                        service.verify(token).await.unwrap(),
                        LnUrlVerifyResponse::Ok { settled: false, preimage: None, pr } if pr == "original-bolt11"
                    ));
                }
            }
        }

        mod when_settled {
            use super::*;
            use crate::application::composition::Ledger;

            #[tokio::test]
            async fn returns_the_preimage_for_lightning_and_internal_settlement() {
                for ledger in [Ledger::Lightning, Ledger::Internal] {
                    let mut invoice = verification_invoice(InvoiceStatus::Settled);
                    invoice.ledger = ledger;
                    let (service, token) = with_invoice(invoice);
                    assert!(matches!(
                        service.verify(token).await.unwrap(),
                        LnUrlVerifyResponse::Ok { settled: true, preimage: Some(preimage), pr }
                            if preimage == "11".repeat(32) && pr == "original-bolt11"
                    ));
                }
            }
        }

        mod when_missing {
            use super::*;

            #[tokio::test]
            async fn returns_an_lnurl_error() {
                let mut store = MockAppStoreBuilder::new();
                store.invoice.expect_find_many().times(1).returning(|_| Ok(vec![]));
                assert!(matches!(
                    service(store, MockLnClient::new()).verify(Uuid::new_v4()).await.unwrap(),
                    LnUrlVerifyResponse::Error { reason } if reason == "Not found"
                ));
            }
        }

        mod when_lookup_fails {
            use super::*;

            #[tokio::test]
            async fn propagates_the_database_error() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .invoice
                    .expect_find_many()
                    .times(1)
                    .returning(|_| Err(DatabaseError::FindMany("unavailable".into())));
                assert!(matches!(
                    service(store, MockLnClient::new()).verify(Uuid::new_v4()).await,
                    Err(ApplicationError::Database(DatabaseError::FindMany(_)))
                ));
            }
        }
    }

    mod lnurlp {
        use super::*;

        mod when_address_is_active {
            use super::*;

            #[tokio::test]
            async fn returns_pay_request_metadata() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .withf(|username| username == "alice")
                    .times(1)
                    .returning(|_| Ok(Some(ln_address(true))));

                let request = service(store, MockLnClient::new())
                    .lnurlp("alice".to_string())
                    .await
                    .unwrap();

                assert_eq!(request.tag, "payRequest");
                assert_eq!(request.min_sendable, MIN_SENDABLE);
                assert_eq!(request.max_sendable, MAX_SENDABLE);
                assert!(request.callback.contains("alice"));
            }
        }

        mod when_address_is_inactive {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Ok(Some(ln_address(false))));

                let err = service(store, MockLnClient::new())
                    .lnurlp("alice".to_string())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }

        mod when_address_is_missing {
            use super::*;

            #[tokio::test]
            async fn returns_not_found() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Ok(None));

                let err = service(store, MockLnClient::new())
                    .lnurlp("alice".to_string())
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }

    mod lnurlp_callback {
        use super::*;

        mod when_amount_is_outside_the_advertised_range {
            use super::*;

            #[tokio::test]
            async fn rejects_the_request_without_calling_the_node() {
                for amount in [0, MIN_SENDABLE - 1, MAX_SENDABLE + 1, i64::MAX as u64 + 1, u64::MAX] {
                    let err = service(MockAppStoreBuilder::new(), MockLnClient::new())
                        .lnurlp_callback("alice".to_string(), amount, None)
                        .await
                        .unwrap_err();

                    assert!(matches!(err, ApplicationError::Data(DataError::Validation(_))));
                }
            }
        }

        mod when_address_is_active {
            use super::*;

            #[tokio::test]
            async fn issues_and_persists_an_invoice() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Ok(Some(ln_address(true))));
                store
                    .invoice
                    .expect_insert()
                    .withf(|invoice| {
                        let lnurl = invoice.lnurl.as_ref().unwrap();
                        let preimage = hex::decode(&lnurl.preimage).unwrap();
                        lnurl.verify_token != invoice.id
                            && invoice.ln_invoice.as_ref().unwrap().payment_hash
                                == sha256::Hash::hash(&preimage).to_string()
                    })
                    .times(1)
                    .returning(Ok);

                let mut ln_client = MockLnClient::new();
                ln_client
                    .expect_invoice()
                    .withf(|_, _, _, _, options| options.description_hash_only && options.preimage.is_some())
                    .times(1)
                    .returning(|_, _, _, _, options| {
                        Ok(Invoice {
                            ln_invoice: Some(LnInvoice {
                                bolt11: "lnbc1example".to_string(),
                                payment_hash: sha256::Hash::hash(&options.preimage.unwrap()).to_string(),
                                ..Default::default()
                            }),
                            ..Default::default()
                        })
                    });

                let callback = service(store, ln_client)
                    .lnurlp_callback("alice".to_string(), 2_000, None)
                    .await
                    .unwrap();

                assert_eq!(callback.pr, "lnbc1example");
                assert!(callback.success_action.is_some());
            }
        }

        mod when_address_is_inactive {
            use super::*;

            #[tokio::test]
            async fn returns_not_found_without_calling_the_node() {
                let mut store = MockAppStoreBuilder::new();
                store
                    .ln_address
                    .expect_find_by_username()
                    .times(1)
                    .returning(|_| Ok(Some(ln_address(false))));

                // ln_client.invoice is intentionally not expected.
                let err = service(store, MockLnClient::new())
                    .lnurlp_callback("alice".to_string(), 2_000, None)
                    .await
                    .unwrap_err();

                assert!(matches!(err, ApplicationError::Data(DataError::NotFound(_))));
            }
        }
    }
}

use std::{sync::Arc, vec};

use async_trait::async_trait;
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    application::{
        composition::AppStore,
        errors::{ApplicationError, DataError},
    },
    infra::lightning::LnClient,
};

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
        let mut invoice = self
            .ln_client
            .invoice(
                amount,
                self.metadata(&username),
                invoice_id.to_string(),
                self.invoice_expiry,
                true,
            )
            .await?;
        invoice.id = invoice_id;
        invoice.wallet_id.clone_from(&ln_address.wallet_id);
        invoice.ln_address_id = Some(ln_address.id);
        invoice.description = Some(comment.unwrap_or(format!("Payment to {}@{}", username, self.domain)));

        // TODO: Get or add more information to make this a LNURLp invoice (like fetching a success action specific to the user)
        let invoice = self.store.invoice.insert(invoice).await?;
        let lnurlp_invoice = LnUrlCallback {
            pr: invoice.ln_invoice.expect("should exist for ledger Lightning").bolt11,
            success_action: Some(LnUrlSuccessAction {
                tag: "message".to_string(),
                message: Some("Thanks for the sats!".to_string()),
                ..Default::default()
            }),
            disposable: None,
            routes: vec![],
        };

        info!(username, "Lightning invoice generated successfully");
        Ok(lnurlp_invoice)
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
                store.invoice.expect_insert().times(1).returning(Ok);

                let mut ln_client = MockLnClient::new();
                ln_client
                    .expect_invoice()
                    .withf(|_, _, _, _, deschashonly| *deschashonly)
                    .times(1)
                    .returning(|_, _, _, _, _| {
                        Ok(Invoice {
                            ln_invoice: Some(LnInvoice {
                                bolt11: "lnbc1example".to_string(),
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

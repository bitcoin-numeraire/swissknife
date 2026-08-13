use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use uuid::Uuid;

use crate::application::{
    composition::AppStore,
    errors::{ApplicationError, DataError},
};

use super::{
    ClientEventType, CreateWebhookSubscriptionRequest, CreatedWebhookSubscription, NewWebhookSubscription,
    RotateWebhookSecretResponse, UpdateWebhookSubscriptionRequest, WebhookDelivery, WebhookSubscription,
    WebhookUseCases,
};

const DELIVERY_HISTORY_LIMIT: u64 = 100;

pub struct WebhookService {
    store: AppStore,
}

impl WebhookService {
    pub fn new(store: AppStore) -> Self {
        Self { store }
    }

    fn generate_secret() -> String {
        let bytes: [u8; 32] = rand::random();
        URL_SAFE_NO_PAD.encode(bytes)
    }

    pub(crate) fn validate_url(url: &str) -> Result<(), DataError> {
        let parsed = reqwest::Url::parse(url)
            .map_err(|_| DataError::Validation("Webhook URL must be a valid HTTPS URL.".to_string()))?;
        if parsed.scheme() != "https" || parsed.host_str().is_none() {
            return Err(DataError::Validation(
                "Webhook URL must use HTTPS and include a host.".to_string(),
            ));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() || parsed.fragment().is_some() {
            return Err(DataError::Validation(
                "Webhook URL cannot include credentials or a fragment.".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_event_types(event_types: Vec<ClientEventType>) -> Result<Vec<ClientEventType>, DataError> {
        if event_types.is_empty() {
            return Err(DataError::Validation(
                "At least one webhook event type is required.".to_string(),
            ));
        }

        let mut unique = Vec::with_capacity(event_types.len());
        for event_type in event_types {
            if !unique.contains(&event_type) {
                unique.push(event_type);
            }
        }
        Ok(unique)
    }

    async fn find_owned(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        id: Uuid,
    ) -> Result<super::StoredWebhookSubscription, ApplicationError> {
        self.store
            .webhook
            .find_owned(account_id, wallet_id, id)
            .await?
            .ok_or_else(|| DataError::NotFound("Webhook subscription not found.".to_string()).into())
    }
}

#[async_trait]
impl WebhookUseCases for WebhookService {
    async fn create(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        request: CreateWebhookSubscriptionRequest,
    ) -> Result<CreatedWebhookSubscription, ApplicationError> {
        if !self.store.wallet.exists_for_account(account_id, wallet_id).await? {
            return Err(DataError::NotFound("Wallet not found.".to_string()).into());
        }
        Self::validate_url(&request.url)?;
        let event_types = Self::validate_event_types(request.event_types)?;

        if self
            .store
            .webhook
            .find_many(account_id, wallet_id)
            .await?
            .iter()
            .any(|subscription| subscription.url == request.url)
        {
            return Err(DataError::Conflict("A webhook already exists for this URL.".to_string()).into());
        }

        let signing_secret = Self::generate_secret();
        let stored = self
            .store
            .webhook
            .insert(NewWebhookSubscription {
                id: Uuid::new_v4(),
                account_id,
                wallet_id,
                url: request.url,
                event_types,
                signing_secret: signing_secret.clone(),
                last_event_id: self.store.client_event.latest_id(wallet_id).await?.unwrap_or_default(),
            })
            .await?;

        Ok(CreatedWebhookSubscription {
            subscription: stored.into(),
            signing_secret,
        })
    }

    async fn list(&self, account_id: Uuid, wallet_id: Uuid) -> Result<Vec<WebhookSubscription>, ApplicationError> {
        if !self.store.wallet.exists_for_account(account_id, wallet_id).await? {
            return Err(DataError::NotFound("Wallet not found.".to_string()).into());
        }

        Ok(self
            .store
            .webhook
            .find_many(account_id, wallet_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn update(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        id: Uuid,
        request: UpdateWebhookSubscriptionRequest,
    ) -> Result<WebhookSubscription, ApplicationError> {
        let mut stored = self.find_owned(account_id, wallet_id, id).await?;
        if let Some(url) = request.url {
            Self::validate_url(&url)?;
            if self
                .store
                .webhook
                .find_many(account_id, wallet_id)
                .await?
                .iter()
                .any(|subscription| subscription.id != id && subscription.url == url)
            {
                return Err(DataError::Conflict("A webhook already exists for this URL.".to_string()).into());
            }
            stored.url = url;
        }
        if let Some(event_types) = request.event_types {
            stored.event_types = Self::validate_event_types(event_types)?;
        }
        if let Some(active) = request.active {
            if stored.active != active {
                stored.last_event_id = self.store.client_event.latest_id(wallet_id).await?.unwrap_or_default();
            }
            stored.active = active;
        }

        let stored = self.store.webhook.update(stored).await?;
        if !stored.active {
            self.store
                .webhook
                .cancel_pending(stored.id, "Subscription disabled.".to_string())
                .await?;
        }
        Ok(stored.into())
    }

    async fn delete(&self, account_id: Uuid, wallet_id: Uuid, id: Uuid) -> Result<(), ApplicationError> {
        if self.store.webhook.delete_owned(account_id, wallet_id, id).await? == 0 {
            return Err(DataError::NotFound("Webhook subscription not found.".to_string()).into());
        }
        Ok(())
    }

    async fn rotate_secret(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        id: Uuid,
    ) -> Result<RotateWebhookSecretResponse, ApplicationError> {
        let mut stored = self.find_owned(account_id, wallet_id, id).await?;
        let signing_secret = Self::generate_secret();
        stored.signing_secret.clone_from(&signing_secret);
        self.store.webhook.update(stored).await?;
        Ok(RotateWebhookSecretResponse { signing_secret })
    }

    async fn list_deliveries(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        subscription_id: Uuid,
    ) -> Result<Vec<WebhookDelivery>, ApplicationError> {
        self.find_owned(account_id, wallet_id, subscription_id).await?;
        Ok(self
            .store
            .webhook
            .list_deliveries(account_id, wallet_id, subscription_id, DELIVERY_HISTORY_LIMIT)
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::application::composition::MockAppStoreBuilder;

    use super::*;
    use crate::domains::event::StoredWebhookSubscription;

    #[test]
    fn validates_https_urls_and_nonempty_event_filters() {
        assert!(WebhookService::validate_url("https://hooks.example.com/swissknife").is_ok());
        for url in [
            "http://hooks.example.com",
            "https://user:password@hooks.example.com",
            "https://hooks.example.com/path#fragment",
            "not a url",
        ] {
            assert!(WebhookService::validate_url(url).is_err(), "{url} must be rejected");
        }
        assert!(WebhookService::validate_event_types(Vec::new()).is_err());
        assert_eq!(
            WebhookService::validate_event_types(vec![
                ClientEventType::InvoicePaid,
                ClientEventType::InvoicePaid,
                ClientEventType::PaymentSettled,
            ])
            .unwrap(),
            vec![ClientEventType::InvoicePaid, ClientEventType::PaymentSettled]
        );
    }

    #[tokio::test]
    async fn creation_starts_after_latest_event_and_returns_secret_once() {
        let account_id = Uuid::new_v4();
        let wallet_id = Uuid::new_v4();
        let mut store = MockAppStoreBuilder::new();
        store
            .wallet
            .expect_exists_for_account()
            .withf(move |account, wallet| *account == account_id && *wallet == wallet_id)
            .times(1)
            .returning(|_, _| Ok(true));
        store
            .webhook
            .expect_find_many()
            .times(1)
            .returning(|_, _| Ok(Vec::new()));
        store
            .client_event
            .expect_latest_id()
            .withf(move |wallet| *wallet == wallet_id)
            .times(1)
            .returning(|_| Ok(Some(42)));
        store
            .webhook
            .expect_insert()
            .withf(move |subscription| {
                subscription.account_id == account_id
                    && subscription.wallet_id == wallet_id
                    && subscription.last_event_id == 42
                    && subscription.signing_secret.len() == 43
            })
            .times(1)
            .returning(|subscription| {
                Ok(StoredWebhookSubscription {
                    id: subscription.id,
                    account_id: subscription.account_id,
                    wallet_id: subscription.wallet_id,
                    url: subscription.url,
                    event_types: subscription.event_types,
                    signing_secret: subscription.signing_secret,
                    active: true,
                    last_event_id: subscription.last_event_id,
                    created_at: Utc::now(),
                    updated_at: None,
                })
            });

        let created = WebhookService::new(store.build())
            .create(
                account_id,
                wallet_id,
                CreateWebhookSubscriptionRequest {
                    url: "https://hooks.example.com/swissknife".to_string(),
                    event_types: vec![ClientEventType::PaymentSettled],
                },
            )
            .await
            .unwrap();

        assert_eq!(created.subscription.wallet_id, wallet_id);
        assert_eq!(URL_SAFE_NO_PAD.decode(created.signing_secret).unwrap().len(), 32);
    }
}

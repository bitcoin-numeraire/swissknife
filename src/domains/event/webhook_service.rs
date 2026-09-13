use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use reqwest::Url;
use tracing::{debug, info, trace};
use uuid::Uuid;

use crate::application::{
    composition::AppStore,
    errors::{ApplicationError, DataError, DatabaseError},
};

use super::{
    ClientEventType, CreateWebhookSubscriptionRequest, CreatedWebhookSubscription, NewWebhookSubscription,
    RotateWebhookSecretResponse, StoredWebhookSubscription, UpdateWebhookSubscriptionRequest, WebhookDelivery,
    WebhookSubscription, WebhookUseCases,
};

const DELIVERY_HISTORY_LIMIT: u64 = 100;
const DUPLICATE_URL_MESSAGE: &str = "A webhook already exists for this URL.";

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

    fn map_write_error(error: DatabaseError) -> ApplicationError {
        match error {
            DatabaseError::Conflict(_) => DataError::Conflict(DUPLICATE_URL_MESSAGE.to_string()).into(),
            error => error.into(),
        }
    }

    pub(crate) fn validate_url(url: &str) -> Result<(), DataError> {
        let parsed =
            Url::parse(url).map_err(|_| DataError::Validation("Webhook URL must be a valid HTTPS URL.".to_string()))?;
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
    ) -> Result<StoredWebhookSubscription, ApplicationError> {
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
        debug!(%account_id, %wallet_id, "Creating webhook subscription");
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
            return Err(DataError::Conflict(DUPLICATE_URL_MESSAGE.to_string()).into());
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
            })
            .await
            .map_err(Self::map_write_error)?;

        info!(%account_id, %wallet_id, subscription_id = %stored.id, "Webhook subscription created successfully");
        Ok(CreatedWebhookSubscription {
            subscription: stored.into(),
            signing_secret,
        })
    }

    async fn list(&self, account_id: Uuid, wallet_id: Uuid) -> Result<Vec<WebhookSubscription>, ApplicationError> {
        trace!(%account_id, %wallet_id, "Listing webhook subscriptions");
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
        debug!(%account_id, %wallet_id, subscription_id = %id, "Updating webhook subscription");
        self.find_owned(account_id, wallet_id, id).await?;
        let mut request = request;
        if let Some(url) = &request.url {
            Self::validate_url(url)?;
            if self
                .store
                .webhook
                .find_many(account_id, wallet_id)
                .await?
                .iter()
                .any(|subscription| subscription.id != id && &subscription.url == url)
            {
                return Err(DataError::Conflict(DUPLICATE_URL_MESSAGE.to_string()).into());
            }
        }
        if let Some(event_types) = request.event_types.take() {
            request.event_types = Some(Self::validate_event_types(event_types)?);
        }
        let stored = self
            .store
            .webhook
            .update(id, request)
            .await
            .map_err(Self::map_write_error)?;
        info!(%account_id, %wallet_id, subscription_id = %id, "Webhook subscription updated successfully");
        Ok(stored.into())
    }

    async fn delete(&self, account_id: Uuid, wallet_id: Uuid, id: Uuid) -> Result<(), ApplicationError> {
        debug!(%account_id, %wallet_id, subscription_id = %id, "Deleting webhook subscription");
        if self.store.webhook.delete_owned(account_id, wallet_id, id).await? == 0 {
            return Err(DataError::NotFound("Webhook subscription not found.".to_string()).into());
        }
        info!(%account_id, %wallet_id, subscription_id = %id, "Webhook subscription deleted successfully");
        Ok(())
    }

    async fn rotate_secret(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        id: Uuid,
    ) -> Result<RotateWebhookSecretResponse, ApplicationError> {
        debug!(%account_id, %wallet_id, subscription_id = %id, "Rotating webhook signing secret");
        self.find_owned(account_id, wallet_id, id).await?;
        let signing_secret = Self::generate_secret();
        self.store.webhook.rotate_secret(id, signing_secret.clone()).await?;
        info!(%account_id, %wallet_id, subscription_id = %id, "Webhook signing secret rotated successfully");
        Ok(RotateWebhookSecretResponse { signing_secret })
    }

    async fn list_deliveries(
        &self,
        account_id: Uuid,
        wallet_id: Uuid,
        subscription_id: Uuid,
    ) -> Result<Vec<WebhookDelivery>, ApplicationError> {
        trace!(%account_id, %wallet_id, %subscription_id, "Listing webhook deliveries");
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

    #[test]
    fn maps_unique_storage_conflicts_to_the_public_duplicate_error() {
        let error = WebhookService::map_write_error(DatabaseError::Conflict("unique violation".to_string()));
        assert!(matches!(
            error,
            ApplicationError::Data(DataError::Conflict(message)) if message == DUPLICATE_URL_MESSAGE
        ));
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
            .webhook
            .expect_insert()
            .withf(move |subscription| {
                subscription.account_id == account_id
                    && subscription.wallet_id == wallet_id
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
                    last_event_id: 42,
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

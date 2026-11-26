use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use rmcp::{
    model::{ResourceUpdatedNotificationParam, SubscriptionFilter},
    service::{Peer, SubscriptionContext},
    ErrorData, RoleServer,
};
use serde_json::Value;
use tokio::{sync::watch, task::JoinHandle};

use crate::wallet::WalletClient;

const POLL_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum WalletResource {
    Balance,
    RecentPayments,
}

impl WalletResource {
    const ALL: [Self; 2] = [Self::Balance, Self::RecentPayments];

    pub(super) fn uri(self) -> &'static str {
        match self {
            Self::Balance => "wallet://balance",
            Self::RecentPayments => "wallet://payments/recent",
        }
    }

    pub(super) fn parse(uri: &str) -> Result<Self, ErrorData> {
        Self::ALL
            .into_iter()
            .find(|r| r.uri() == uri)
            .ok_or_else(|| ErrorData::resource_not_found("Unknown wallet resource", None))
    }

    pub(super) async fn read(self, wallet: &WalletClient) -> anyhow::Result<Value> {
        match self {
            Self::Balance => wallet.get("balance").await,
            Self::RecentPayments => wallet.recent(Default::default()).await,
        }
    }
}

fn retryable_read(error: &anyhow::Error) -> bool {
    let Some(http) = error.downcast_ref::<reqwest::Error>() else {
        return false;
    };
    match http.status() {
        Some(status) => status.is_server_error() || matches!(status.as_u16(), 408 | 429),
        // reqwest classifies interrupted response frames from Response::chunk as decode errors.
        None => http.is_timeout() || http.is_connect() || http.is_request() || http.is_body() || http.is_decode(),
    }
}

#[derive(Clone, Default)]
pub(super) struct ResourceFeeds {
    active: Arc<Mutex<HashMap<WalletResource, Weak<watch::Receiver<()>>>>>,
}

struct Updates {
    // The last subscriber dropping this reference also stops the wallet poller.
    _feed: Arc<watch::Receiver<()>>,
    updates: watch::Receiver<()>,
}

impl Updates {
    async fn changed(&mut self) -> bool {
        self.updates.changed().await.is_ok()
    }
}

impl ResourceFeeds {
    async fn watch(&self, resource: WalletResource, wallet: &WalletClient) -> Result<Updates, ErrorData> {
        let mut previous = resource
            .read(wallet)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let mut active = self.active.lock().expect("resource feeds lock");
        let existing = active
            .get(&resource)
            .and_then(Weak::upgrade)
            .filter(|r| r.has_changed().is_ok());
        let feed = existing.unwrap_or_else(|| {
            let (tx, rx) = watch::channel(());
            let rx = Arc::new(rx);
            active.insert(resource, Arc::downgrade(&rx));
            let wallet = wallet.clone();
            tokio::spawn(async move {
                let mut delay = POLL_INTERVAL;
                loop {
                    tokio::select! {
                        _ = tx.closed() => return,
                        _ = tokio::time::sleep(delay) => {}
                    }
                    let current = tokio::select! {
                        _ = tx.closed() => return,
                        result = resource.read(&wallet) => match result {
                            Ok(current) => current,
                            Err(error) if retryable_read(&error) => {
                                delay = (delay * 2).min(Duration::from_secs(30));
                                continue;
                            }
                            Err(_) => return,
                        }
                    };
                    delay = POLL_INTERVAL;
                    if current != previous {
                        previous = current;
                        if tx.send(()).is_err() {
                            return;
                        }
                    }
                }
            });
            rx
        });
        let mut updates = feed.as_ref().clone();
        updates.borrow_and_update();
        Ok(Updates { _feed: feed, updates })
    }

    pub(super) fn filter(requested: &SubscriptionFilter) -> SubscriptionFilter {
        let requested = requested.resource_subscriptions.as_deref().unwrap_or_default();
        SubscriptionFilter::builder()
            .resource_subscriptions(
                WalletResource::ALL
                    .into_iter()
                    .map(WalletResource::uri)
                    .filter(|uri| requested.iter().any(|r| r == uri)),
            )
            .build()
    }

    pub(super) async fn listen(&self, wallet: &WalletClient, context: SubscriptionContext) -> Result<(), ErrorData> {
        let mut listeners = tokio::task::JoinSet::new();
        for uri in context.accepted().resource_subscriptions.iter().flatten() {
            let resource = WalletResource::parse(uri)?;
            let mut subscription = self.watch(resource, wallet).await?;
            let sink = context.sink().clone();
            // The SDK acknowledges a listen request before entering this handler.
            // An initial invalidation lets the host read after the feed is attached.
            sink.notify_resource_updated(resource.uri())
                .await
                .map_err(|_| ErrorData::internal_error("Resource subscription closed", None))?;
            listeners.spawn(async move {
                while subscription.changed().await {
                    if sink.notify_resource_updated(resource.uri()).await.is_err() {
                        break;
                    }
                }
            });
        }
        if !listeners.is_empty() {
            tokio::select! {
                _ = context.cancelled() => {}
                _ = listeners.join_next() => {}
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct LegacySubscriptions(Mutex<HashMap<WalletResource, JoinHandle<()>>>);

impl LegacySubscriptions {
    pub(super) async fn subscribe(
        &self,
        resource: WalletResource,
        feeds: &ResourceFeeds,
        wallet: &WalletClient,
        peer: Peer<RoleServer>,
    ) -> Result<(), ErrorData> {
        let mut subscription = feeds.watch(resource, wallet).await?;
        let mut tasks = self.0.lock().expect("legacy subscriptions lock");
        if tasks.get(&resource).is_some_and(|task| !task.is_finished()) {
            return Ok(());
        }
        tasks.insert(
            resource,
            tokio::spawn(async move {
                while subscription.changed().await {
                    if peer
                        .notify_resource_updated(ResourceUpdatedNotificationParam::new(resource.uri()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }),
        );
        Ok(())
    }

    pub(super) async fn unsubscribe(&self, resource: WalletResource) {
        let task = self.0.lock().expect("legacy subscriptions lock").remove(&resource);
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
        }
    }
}

impl Drop for LegacySubscriptions {
    fn drop(&mut self) {
        for task in self.0.get_mut().expect("legacy subscriptions lock").values() {
            task.abort();
        }
    }
}

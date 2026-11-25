use super::{CreateOfferRequest, OfferFilter, OfferState, WalletOffer};
use crate::{
    application::{
        composition::AppStore,
        errors::{ApplicationError, DataError, LightningError},
    },
    domains::bitcoin::BtcNetwork,
    infra::lightning::LnClient,
};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OfferUseCases: Send + Sync {
    async fn create(&self, wallet: Uuid, request: CreateOfferRequest) -> Result<WalletOffer, ApplicationError>;
    async fn list(&self, filter: OfferFilter) -> Result<Vec<WalletOffer>, ApplicationError>;
    async fn disable(&self, id: Uuid, wallet: Option<Uuid>) -> Result<WalletOffer, ApplicationError>;
    async fn sync(&self) -> Result<bool, ApplicationError>;
}

pub struct OfferService {
    store: AppStore,
    node: Arc<dyn LnClient>,
    network: BtcNetwork,
}
impl OfferService {
    pub fn new(store: AppStore, node: Arc<dyn LnClient>, network: BtcNetwork) -> Self {
        Self { store, node, network }
    }
    fn backend(&self) -> Result<String, ApplicationError> {
        self.node.offer_backend().ok_or_else(|| {
            LightningError::InvalidPaymentTarget("This node does not support receiving BOLT12 offers".into()).into()
        })
    }
    fn require_original(&self, backend: &str) -> Result<(), ApplicationError> {
        if self.node.offer_backend().as_deref() != Some(backend) {
            return Err(
                DataError::Conflict("Reconnect the original Core Lightning node to manage this offer".into()).into(),
            );
        }
        Ok(())
    }
    async fn finish_registration(&self, offer: WalletOffer, fresh: bool) -> Result<WalletOffer, ApplicationError> {
        let label = offer.id.to_string();
        // CLN uses fresh blinded paths. Recover an earlier registration by its private node label.
        let existing = if fresh {
            None
        } else {
            self.node
                .local_offers(None)
                .await?
                .into_iter()
                .find(|l| l.label.as_deref() == Some(&label) && l.active)
        };
        let issued = match existing {
            Some(existing) => existing,
            None => self.node.issue_offer(&offer).await?,
        };
        let saved = self.store.offer.activate(offer.id, &issued).await?.offer;
        if saved.provider_id.as_deref() != Some(&issued.offer_id) {
            self.node.disable_offer(&issued.offer_id).await?;
        }
        Ok(saved)
    }
}
#[async_trait]
impl OfferUseCases for OfferService {
    async fn create(&self, wallet: Uuid, request: CreateOfferRequest) -> Result<WalletOffer, ApplicationError> {
        let backend = self.backend()?;
        if request.idempotency_key.is_nil()
            || request.description.trim().is_empty()
            || request.description.len() > 256
            || request.amount_msat.is_some_and(|v| v == 0 || v > i64::MAX as u64)
        {
            return Err(DataError::Validation(
                "Supply a request key, a description of 1–256 bytes and a positive millisatoshi amount if fixed".into(),
            )
            .into());
        }
        let asset = self
            .store
            .wallet
            .find(wallet)
            .await?
            .ok_or_else(|| DataError::NotFound("Wallet not found".into()))?
            .asset;
        if !asset.is_some_and(|a| {
            a.protocol == swissknife_types::Protocol::Bitcoin
                && a.asset_ref == swissknife_types::NATIVE_ASSET_REF
                && a.network == self.network
        }) {
            return Err(
                DataError::Validation("A native BTC wallet on the connected node network is required".into()).into(),
            );
        }
        let (stored, fresh) = self.store.offer.register(wallet, &backend, &request).await?;
        let offer = stored.offer;
        if offer.description != request.description
            || offer.single_use != request.single_use
            || offer.amount_msat != request.amount_msat
        {
            return Err(DataError::Conflict("The offer request key was used for different terms".into()).into());
        }
        if offer.state != OfferState::Creating {
            return Ok(offer);
        }
        self.require_original(&stored.backend)?;
        self.finish_registration(offer, fresh).await
    }
    async fn list(&self, filter: OfferFilter) -> Result<Vec<WalletOffer>, ApplicationError> {
        self.store.offer.list(filter).await
    }
    async fn disable(&self, id: Uuid, wallet: Option<Uuid>) -> Result<WalletOffer, ApplicationError> {
        let stored = self.store.offer.get(id, wallet).await?;
        if stored.offer.state == OfferState::Disabled {
            return Ok(stored.offer);
        }
        self.require_original(&stored.backend)?;
        let provider_id = stored
            .offer
            .provider_id
            .ok_or_else(|| DataError::Conflict("Retry creation before disabling this offer".into()))?;
        self.store
            .offer
            .transition(id, OfferState::Active, OfferState::Disabling)
            .await?;
        if self
            .node
            .local_offers(Some(provider_id.clone()))
            .await?
            .iter()
            .any(|offer| offer.active)
        {
            self.node.disable_offer(&provider_id).await?;
        }
        Ok(self
            .store
            .offer
            .transition(id, OfferState::Disabling, OfferState::Disabled)
            .await?
            .offer)
    }
    async fn sync(&self) -> Result<bool, ApplicationError> {
        let Some(backend) = self.node.offer_backend() else {
            return Ok(false);
        };
        if let Some(stored) = self.store.offer.pending(&backend).await? {
            let result = if stored.offer.state == OfferState::Creating {
                self.finish_registration(stored.offer, false).await
            } else {
                self.disable(stored.offer.id, None).await
            };
            // An operator-side failure managing one offer must not block incoming credits.
            if let Err(error) = result {
                tracing::warn!(%error,"Offer management will retry")
            }
        }
        let Some(after) = self.store.offer.cursor(&backend).await? else {
            return Ok(false);
        };
        let mut page = self.node.offer_updates(after + 1).await?;
        for update in &mut page {
            let Some(id) = update.local_offer_id.as_deref() else {
                continue;
            };
            if update.status != "paid" || self.store.offer.owner(id).await?.is_none() {
                continue;
            }
            let invoice = update
                .bolt12
                .as_deref()
                .ok_or_else(|| DataError::Inconsistency("Missing paid BOLT12 invoice".into()))?;
            let decoded = self.node.decode_bolt12(invoice).await?;
            if !decoded.valid
                || decoded.kind != "bolt12 invoice"
                || decoded.offer_id.as_deref() != Some(id)
                || decoded.invoice_payment_hash.as_deref() != Some(&update.payment_hash)
            {
                return Err(DataError::Inconsistency(
                    "Paid invoice does not match its Core Lightning offer receipt".into(),
                )
                .into());
            }
            update.invoice_created_at = decoded.invoice_created_at;
        }
        if page.is_empty() {
            return Ok(false);
        }
        let full = page.len() == 100;
        self.store.offer.project(&backend, after, page).await?;
        Ok(full)
    }
}

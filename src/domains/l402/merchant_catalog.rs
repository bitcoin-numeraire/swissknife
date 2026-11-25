use std::{collections::BTreeMap, sync::Arc};

use crate::application::errors::{ApplicationError, DataError, DatabaseError};
use async_trait::async_trait;
use swissknife_types::{MerchantOrigin, MerchantSaleTerms, WalletMerchantCatalog, WalletMerchantRoute};
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait MerchantRouteRepository: Send + Sync {
    async fn list(&self, wallet: Uuid) -> Result<Vec<WalletMerchantRoute>, DatabaseError>;
    async fn get(&self, wallet: Uuid, name: &str) -> Result<Option<MerchantSaleTerms>, DatabaseError>;
    /// A published name keeps its original resource even across competing updates.
    async fn set(&self, wallet: Uuid, name: &str, terms: &MerchantSaleTerms) -> Result<bool, DatabaseError>;
}

pub struct MerchantCatalog {
    pub origins: BTreeMap<(Uuid, String), MerchantOrigin>,
    pub routes: Arc<dyn MerchantRouteRepository>,
}

pub(crate) fn valid_name(name: &str, maximum: usize) -> bool {
    (1..=maximum).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl MerchantCatalog {
    pub async fn list(&self, wallet: Uuid) -> Result<WalletMerchantCatalog, ApplicationError> {
        let origins = self
            .origins
            .iter()
            .filter(|((owner, _), _)| *owner == wallet)
            .map(|(_, origin)| origin.clone())
            .collect();
        Ok(WalletMerchantCatalog {
            origins,
            routes: self.routes.list(wallet).await?,
        })
    }
    pub async fn set(&self, wallet: Uuid, name: &str, terms: MerchantSaleTerms) -> Result<(), ApplicationError> {
        if !valid_name(name, 57) {
            return Err(DataError::Validation(
                "Choose a route name of 1–57 lowercase letters, digits or hyphens".into(),
            )
            .into());
        }
        let origin = self
            .origins
            .get(&(wallet, terms.origin.clone()))
            .ok_or_else(|| DataError::NotFound("Merchant origin not found".into()))?;
        if !(1..=origin.maximum_price_msat).contains(&terms.price_msat) {
            return Err(DataError::Validation("Price must be positive and within the origin ceiling".into()).into());
        }
        if !self.routes.set(wallet, name, &terms).await? {
            return Err(DataError::Conflict("This route name is already assigned to another origin".into()).into());
        }
        Ok(())
    }
}

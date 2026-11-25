use super::models::{
    merchant_route::{ActiveModel, Column, Model},
    prelude::MerchantRoute as Entity,
};
use crate::{application::errors::DatabaseError, domains::l402::MerchantRouteRepository};
use async_trait::async_trait;
use sea_orm::{
    sea_query::{Expr, OnConflict},
    ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, QueryFilter, QueryOrder, Set,
};
use swissknife_types::{MerchantSaleTerms, WalletMerchantRoute};
use uuid::Uuid;

pub struct SeaOrmMerchantRouteRepository(pub DatabaseConnection);

fn terms(row: Model) -> Result<MerchantSaleTerms, DatabaseError> {
    Ok(MerchantSaleTerms {
        origin: row.origin,
        price_msat: u64::try_from(row.price_msat)
            .map_err(|_| DatabaseError::FindOne("Negative merchant price".into()))?,
        enabled: row.enabled,
    })
}

#[async_trait]
impl MerchantRouteRepository for SeaOrmMerchantRouteRepository {
    async fn get(&self, wallet: Uuid, name: &str) -> Result<Option<MerchantSaleTerms>, DatabaseError> {
        Entity::find_by_id((wallet, name.to_owned()))
            .one(&self.0)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
            .map(terms)
            .transpose()
    }
    async fn list(&self, wallet: Uuid) -> Result<Vec<WalletMerchantRoute>, DatabaseError> {
        Entity::find()
            .filter(Column::WalletId.eq(wallet))
            .order_by_asc(Column::Name)
            .all(&self.0)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?
            .into_iter()
            .map(|row| {
                Ok(WalletMerchantRoute {
                    name: row.name.clone(),
                    terms: terms(row)?,
                })
            })
            .collect()
    }
    async fn set(&self, wallet: Uuid, name: &str, terms: &MerchantSaleTerms) -> Result<bool, DatabaseError> {
        let price_msat = i64::try_from(terms.price_msat)
            .map_err(|_| DatabaseError::Update("Merchant price exceeds storage range".into()))?;
        let updated = Entity::insert(ActiveModel {
            wallet_id: Set(wallet),
            name: Set(name.to_owned()),
            origin: Set(terms.origin.clone()),
            price_msat: Set(price_msat),
            enabled: Set(terms.enabled),
        })
        .on_conflict(
            OnConflict::columns([Column::WalletId, Column::Name])
                .update_columns([Column::PriceMsat, Column::Enabled])
                .action_and_where(Expr::col((Entity, Column::Origin)).eq(terms.origin.clone()))
                .to_owned(),
        )
        .exec_without_returning(&self.0)
        .await
        .map_err(|e| DatabaseError::Update(e.to_string()))?;
        Ok(updated == 1)
    }
}

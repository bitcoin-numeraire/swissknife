use super::models::l402_usage::{ActiveModel, Column, Entity};
use crate::{application::errors::DatabaseError, domains::l402::RequestMeter};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::{Expr, ExprTrait, OnConflict},
    DatabaseConnection, EntityTrait, Set,
};
use std::num::NonZeroU32;
use uuid::Uuid;

pub struct SeaOrmRequestMeter(pub DatabaseConnection);

#[async_trait]
impl RequestMeter for SeaOrmRequestMeter {
    async fn used(&self, payment_hash: [u8; 32]) -> Result<u64, DatabaseError> {
        let row = Entity::find_by_id(hex::encode(payment_hash))
            .one(&self.0)
            .await
            .map_err(|error| DatabaseError::FindOne(error.to_string()))?;
        u64::try_from(row.map_or(0, |r| r.requests)).map_err(|_| DatabaseError::FindOne("Negative L402 usage".into()))
    }

    async fn consume(
        &self,
        payment_hash: [u8; 32],
        wallet_id: Uuid,
        route: &str,
        limit: NonZeroU32,
    ) -> Result<bool, DatabaseError> {
        // Qualify the existing row to distinguish it from EXCLUDED on Postgres.
        let requests = Expr::col((Entity, Column::Requests));
        let conflict = OnConflict::column(Column::PaymentHash)
            .value(Column::Requests, requests.clone().add(1))
            .action_and_where(requests.lt(i64::from(limit.get())))
            .to_owned();
        let changed = Entity::insert(ActiveModel {
            payment_hash: Set(hex::encode(payment_hash)),
            wallet_id: Set(wallet_id),
            route: Set(route.to_owned()),
            requests: Set(1),
            first_used_at: Set(Utc::now().naive_utc()),
        })
        .on_conflict(conflict)
        .exec_without_returning(&self.0)
        .await
        .map_err(|error| DatabaseError::Update(error.to_string()))?;
        Ok(changed == 1)
    }
}

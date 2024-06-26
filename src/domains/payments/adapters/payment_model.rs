//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;

use crate::domains::payments::entities::Payment;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "payment")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: String,
    pub ln_address: Option<String>,
    pub payment_hash: Option<String>,
    pub payment_preimage: Option<String>,
    pub error: Option<String>,
    pub amount_msat: i64,
    pub fee_msat: Option<i64>,
    pub payment_time: Option<DateTimeUtc>,
    pub status: String,
    pub ledger: String,
    pub description: Option<String>,
    pub metadata: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub success_action: Option<Json>,
    pub created_at: DateTimeUtc,
    pub updated_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for Payment {
    fn from(model: Model) -> Self {
        Payment {
            id: model.id,
            user_id: model.user_id,
            ln_address: model.ln_address,
            payment_hash: model.payment_hash,
            payment_preimage: model.payment_preimage,
            error: model.error,
            amount_msat: model.amount_msat as u64,
            fee_msat: model.fee_msat.map(|v| v as u64),
            payment_time: model.payment_time,
            status: model.status.parse().unwrap(),
            ledger: model.ledger.parse().unwrap(),
            description: model.description,
            metadata: model.metadata,
            success_action: serde_json::from_value(model.success_action.unwrap_or_default()).ok(),
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

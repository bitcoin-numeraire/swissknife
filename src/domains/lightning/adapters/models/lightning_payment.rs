//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;

use crate::domains::lightning::entities::{LightningPayment, LightningPaymentStatus};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "lightning_payment")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: String,
    pub lightning_address: Option<String>,
    #[sea_orm(unique)]
    pub payment_hash: Option<String>,
    pub error: Option<String>,
    pub amount_msat: i64,
    pub fee_msat: Option<i64>,
    pub payment_time: Option<DateTimeUtc>,
    pub status: String,
    pub description: Option<String>,
    pub metadata: Option<String>,
    pub success_action: Option<serde_json::Value>,
    pub created_at: DateTimeUtc,
    pub updated_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::lightning_address::Entity",
        from = "Column::LightningAddress",
        to = "super::lightning_address::Column::Username",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    LightningAddress,
}

impl Related<super::lightning_address::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LightningAddress.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for LightningPayment {
    fn from(model: Model) -> Self {
        LightningPayment {
            id: model.id,
            user_id: model.user_id,
            lightning_address: model.lightning_address,
            payment_hash: model.payment_hash,
            error: model.error,
            amount_msat: model.amount_msat as u64,
            fee_msat: model.fee_msat.map(|v| v as u64),
            payment_time: model.payment_time,
            status: model.status.parse::<LightningPaymentStatus>().unwrap(),
            description: model.description,
            metadata: model.metadata,
            success_action: model.success_action,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

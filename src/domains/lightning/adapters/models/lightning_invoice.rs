//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;

use crate::domains::lightning::entities::LightningInvoice;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "lightning_invoice")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub lightning_address: Option<String>,
    #[sea_orm(unique)]
    pub bolt11: String,
    pub network: String,
    pub payee_pubkey: String,
    #[sea_orm(unique)]
    pub payment_hash: String,
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub amount_msat: Option<i64>,
    #[sea_orm(column_type = "Binary(BlobSize::Blob(None))")]
    pub payment_secret: Vec<u8>,
    pub timestamp: i64,
    pub expiry: i64,
    pub min_final_cltv_expiry_delta: i64,
    pub status: String,
    pub fee_msat: Option<i64>,
    pub payment_time: Option<i64>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: Option<DateTimeWithTimeZone>,
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

impl From<Model> for LightningInvoice {
    fn from(model: Model) -> Self {
        LightningInvoice {
            id: model.id,
            lightning_address: model.lightning_address,
            bolt11: model.bolt11,
            network: model.network,
            payee_pubkey: model.payee_pubkey,
            payment_hash: model.payment_hash,
            description: model.description,
            description_hash: model.description_hash,
            amount_msat: model.amount_msat.map(|v| v as u64),
            payment_secret: model.payment_secret,
            min_final_cltv_expiry_delta: model.min_final_cltv_expiry_delta as u64,
            timestamp: model.timestamp as u64,
            expiry: model.expiry as u64,
            status: model.status,
            fee_msat: None,
            payment_time: model.payment_time,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
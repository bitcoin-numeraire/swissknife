//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "lightning_payments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub lightning_address: Option<String>,
    #[sea_orm(unique)]
    pub payment_hash: String,
    pub error: Option<String>,
    pub amount_msat: i64,
    pub fee_msat: Option<i64>,
    pub payment_time: Option<i64>,
    pub status: String,
    pub description: Option<String>,
    pub metadata: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::lightning_addresses::Entity",
        from = "Column::LightningAddress",
        to = "super::lightning_addresses::Column::Username",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    LightningAddresses,
}

impl Related<super::lightning_addresses::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LightningAddresses.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

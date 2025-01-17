//! `SeaORM` Entity, @generated by sea-orm-codegen 1.0.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "payment")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub ln_address: Option<String>,
    #[sea_orm(unique)]
    pub payment_hash: Option<String>,
    #[sea_orm(unique)]
    pub payment_preimage: Option<String>,
    pub error: Option<String>,
    pub amount_msat: i64,
    pub fee_msat: Option<i64>,
    pub payment_time: Option<DateTimeUtc>,
    pub status: String,
    pub ledger: String,
    pub currency: String,
    pub description: Option<String>,
    pub metadata: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub success_action: Option<Json>,
    pub created_at: DateTimeUtc,
    pub updated_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::wallet::Entity",
        from = "Column::WalletId",
        to = "super::wallet::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Wallet,
}

impl Related<super::wallet::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Wallet.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

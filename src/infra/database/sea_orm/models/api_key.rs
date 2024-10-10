//! `SeaORM` Entity, @generated by sea-orm-codegen 1.0.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "api_key")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: String,
    #[sea_orm(column_type = "VarBinary(32)", unique)]
    pub key_hash: Vec<u8>,
    pub permissions: Vec<String>,
    pub created_at: DateTimeUtc,
    pub expires_at: Option<DateTimeUtc>,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::wallet::Entity",
        from = "Column::UserId",
        to = "super::wallet::Column::UserId",
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
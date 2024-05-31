//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use std::time::Duration;

use chrono::Utc;
use sea_orm::entity::prelude::*;

use crate::domains::invoices::entities::{Invoice, InvoiceStatus, LnInvoice};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "invoice")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: String,
    pub currency: String,
    pub ledger: String,
    pub network: Option<String>,
    pub payment_hash: Option<String>,
    pub ln_address: Option<Uuid>,
    pub bolt11: Option<String>,
    pub payee_pubkey: Option<String>,
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub amount_msat: Option<i64>,
    pub payment_secret: Option<String>,
    pub timestamp: DateTimeUtc,
    pub expiry: Option<i64>,
    pub min_final_cltv_expiry_delta: Option<i64>,
    pub fee_msat: Option<i64>,
    pub payment_time: Option<DateTimeUtc>,
    pub label: Option<Uuid>,
    pub created_at: DateTimeUtc,
    pub updated_at: Option<DateTimeUtc>,
    pub expires_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::domains::lightning::adapters::ln_address_model::Entity",
        from = "Column::LnAddress",
        to = "crate::domains::lightning::adapters::ln_address_model::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    LightningAddress,
}

impl Related<crate::domains::lightning::adapters::ln_address_model::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LightningAddress.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for Invoice {
    fn from(model: Model) -> Self {
        let status = match model.payment_time {
            Some(_) => InvoiceStatus::SETTLED,
            None => match model.expires_at {
                Some(expires_at) if Utc::now() > expires_at => InvoiceStatus::EXPIRED,
                _ => InvoiceStatus::PENDING,
            },
        };

        let lightning = match model.ledger.as_str() {
            "LIGHTNING" => Some(LnInvoice {
                payment_hash: model.payment_hash.unwrap(),
                bolt11: model.bolt11.unwrap(),
                description_hash: model.description_hash,
                payee_pubkey: model.payee_pubkey.unwrap(),
                min_final_cltv_expiry_delta: model.min_final_cltv_expiry_delta.unwrap() as u64,
                payment_secret: model.payment_secret.unwrap(),
                network: model.network.unwrap().parse().unwrap(),
                expiry: Duration::from_secs(model.expiry.unwrap() as u64),
                expires_at: model.expires_at.unwrap(),
            }),
            _ => None,
        };

        Invoice {
            id: model.id,
            user_id: model.user_id,
            ln_address: model.ln_address,
            description: model.description,
            amount_msat: model.amount_msat.map(|v| v as u64),
            timestamp: model.timestamp,
            currency: model.currency.parse().unwrap(),
            ledger: model.ledger.parse().unwrap(),
            status,
            fee_msat: None,
            payment_time: model.payment_time,
            label: model.label,
            created_at: model.created_at,
            updated_at: model.updated_at,
            lightning,
        }
    }
}

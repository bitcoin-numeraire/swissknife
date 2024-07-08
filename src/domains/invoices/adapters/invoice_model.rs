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
    pub payment_hash: Option<String>,
    pub ln_address_id: Option<Uuid>,
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
    pub created_at: DateTimeUtc,
    pub updated_at: Option<DateTimeUtc>,
    pub expires_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::domains::lightning::adapters::ln_address_model::Entity",
        from = "Column::LnAddressId",
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

const ASSERTION_MSG: &str = "should parse successfully by assertion";

impl From<Model> for Invoice {
    fn from(model: Model) -> Self {
        let status = match model.payment_time {
            Some(_) => InvoiceStatus::Settled,
            None => match model.expires_at {
                Some(expires_at) if Utc::now() > expires_at => InvoiceStatus::Expired,
                _ => InvoiceStatus::Pending,
            },
        };

        let lightning = match model.ledger.as_str() {
            "Lightning" => Some(LnInvoice {
                payment_hash: model.payment_hash.expect(ASSERTION_MSG),
                bolt11: model.bolt11.expect(ASSERTION_MSG),
                description_hash: model.description_hash,
                payee_pubkey: model.payee_pubkey.expect(ASSERTION_MSG),
                min_final_cltv_expiry_delta: model.min_final_cltv_expiry_delta.expect(ASSERTION_MSG)
                    as u64,
                payment_secret: model.payment_secret.expect(ASSERTION_MSG),
                expiry: Duration::from_secs(model.expiry.expect(ASSERTION_MSG) as u64),
                expires_at: model.expires_at.expect(ASSERTION_MSG),
            }),
            _ => None,
        };

        Invoice {
            id: model.id,
            user_id: model.user_id,
            ln_address_id: model.ln_address_id,
            description: model.description,
            amount_msat: model.amount_msat.map(|v| v as u64),
            timestamp: model.timestamp,
            currency: model.currency.parse().expect(ASSERTION_MSG),
            ledger: model.ledger.parse().expect(ASSERTION_MSG),
            status,
            fee_msat: None,
            payment_time: model.payment_time,
            created_at: model.created_at,
            updated_at: model.updated_at,
            lightning,
        }
    }
}

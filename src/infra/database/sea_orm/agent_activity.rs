use super::models::{spending_entry as entry, wallet_agent_key as key};
use crate::application::errors::DatabaseError;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use futures_util::TryStreamExt;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::BTreeMap;
use swissknife_types::{AgentActivityPage, AgentSpendingActivity};
use uuid::Uuid;

pub(super) async fn page(
    db: &DatabaseConnection,
    account: Uuid,
    as_of: DateTime<Utc>,
    after: Option<Uuid>,
    limit: u32,
) -> Result<AgentActivityPage, DatabaseError> {
    let fail = |e: sea_orm::DbErr| DatabaseError::FindMany(e.to_string());
    let end = as_of.naive_utc();
    let hour = Duration::hours(1);
    let mut query = entry::Entity::find()
        .select_only()
        .column(entry::Column::AgentKeyId)
        .column(entry::Column::WalletId)
        .column(entry::Column::AdmittedAt)
        .column(entry::Column::ChargedMsat)
        .filter(entry::Column::AccountId.eq(account))
        .filter(entry::Column::AgentKeyId.is_not_null())
        .filter(entry::Column::AdmittedAt.gt(end - hour * 2))
        .filter(entry::Column::AdmittedAt.lte(end))
        .order_by_asc(entry::Column::AgentKeyId);
    if let Some(after) = after {
        query = query.filter(entry::Column::AgentKeyId.gt(after));
    }
    let selector = query.into_tuple::<(Uuid, Option<Uuid>, NaiveDateTime, i64)>();
    let mut rows = selector.stream(db).await.map_err(fail)?;
    let mut agents = BTreeMap::new();
    let mut next_after = None;
    while let Some((id, wallet_id, at, charge)) = rows.try_next().await.map_err(fail)? {
        if agents.len() == limit as usize && !agents.contains_key(&id) {
            next_after = agents.last_key_value().map(|(id, _)| *id);
            break;
        }
        let row = agents.entry(id).or_insert_with(|| AgentSpendingActivity {
            agent_key_id: id,
            wallet_id,
            name: None,
            current_hour: Default::default(),
            previous_hour: Default::default(),
        });
        let bucket = if at > end - hour {
            &mut row.current_hour
        } else {
            &mut row.previous_hour
        };
        bucket.payments += 1;
        bucket.charged_msat +=
            u128::try_from(charge).map_err(|_| DatabaseError::FindMany("Negative agent spending charge".into()))?;
    }
    // SQLite can use a single connection. Release the stream before querying labels.
    drop(rows);
    if !agents.is_empty() {
        let labels = key::Entity::find()
            .select_only()
            .column(key::Column::Id)
            .column(key::Column::Name)
            .filter(key::Column::AccountId.eq(account))
            .filter(key::Column::Id.is_in(agents.keys().copied()))
            .into_tuple::<(Uuid, String)>()
            .all(db)
            .await
            .map_err(fail)?;
        for (id, name) in labels {
            if let Some(row) = agents.get_mut(&id) {
                row.name = Some(name);
            }
        }
    }
    Ok(AgentActivityPage {
        as_of,
        items: agents.into_values().collect(),
        next_after,
    })
}

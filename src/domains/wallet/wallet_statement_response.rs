use axum::{
    http::header,
    response::{IntoResponse, Response},
};
use chrono::SecondsFormat;
use csv::{Terminator, WriterBuilder};

use super::WalletStatement;
use crate::application::errors::{ApplicationError, DataError};

pub(super) fn statement_response(statement: WalletStatement) -> Result<Response, ApplicationError> {
    let mut writer = WriterBuilder::new()
        .terminator(Terminator::CRLF)
        .from_writer(Vec::new());
    let encode_error = |err: csv::Error| DataError::Inconsistency(err.to_string());
    writer
        .write_record([
            "transaction_id",
            "wallet_id",
            "asset_id",
            "network",
            "created_at_utc",
            "payment_time_utc",
            "direction",
            "ledger",
            "status",
            "gross_amount_msat",
            "fee_msat",
            "description",
        ])
        .map_err(encode_error)?;
    let wallet = statement.wallet_id.to_string();
    let asset = statement.asset_id.to_string();
    for entry in statement.entries {
        let description = entry.description.as_deref().unwrap_or_default();
        // Spreadsheet applications can evaluate leading formulas even in a quoted CSV cell.
        let safe_description = if description.trim_start().starts_with(['=', '+', '-', '@'])
            || description.starts_with(['\t', '\n', '\r'])
        {
            format!("'{description}")
        } else {
            description.to_owned()
        };
        writer
            .write_record([
                &entry.id.to_string(),
                &wallet,
                &asset,
                &statement.network,
                &entry.created_at.to_rfc3339_opts(SecondsFormat::AutoSi, true),
                &entry
                    .payment_time
                    .map(|time| time.to_rfc3339_opts(SecondsFormat::AutoSi, true))
                    .unwrap_or_default(),
                &entry.direction,
                &entry.ledger,
                &entry.status,
                &entry.amount_msat.map(|n| n.to_string()).unwrap_or_default(),
                &entry.fee_msat.map(|n| n.to_string()).unwrap_or_default(),
                &safe_description,
            ])
            .map_err(encode_error)?;
    }
    let body = writer
        .into_inner()
        .map_err(|err| DataError::Inconsistency(err.to_string()))?;
    let disposition = format!("attachment; filename=\"wallet-{wallet}.csv\"");
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CONTENT_DISPOSITION, &disposition),
            (header::CACHE_CONTROL, "private, no-store"),
        ],
        body,
    )
        .into_response())
}

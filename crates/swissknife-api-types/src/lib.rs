//! Shared API/contract types for Numeraire SwissKnife.
//!
//! These are pure data — entities, enums, and request types — and are the
//! single source of truth for the backend, the integration tests, and external
//! clients/SDKs. They carry their own serde + `utoipa::ToSchema` annotations so
//! the wire shape lives with the type; behaviour stays in the app's use cases.

mod network;
mod permission;
mod transaction;

pub use network::BtcNetwork;
pub use permission::Permission;
pub use transaction::{Currency, Ledger};

//! Shared API/contract types for Numeraire SwissKnife.
//!
//! These are pure data — entities, shared enums, and request/response types —
//! and are the single source of truth for the backend, the integration tests,
//! and external clients/SDKs. They carry their own serde + `utoipa::ToSchema`
//! annotations so the wire shape lives with the type; behaviour stays in the
//! app's use cases.

mod account;
mod api_key;
mod auth;
mod bitcoin;
mod error;
mod invoice;
mod ln_address;
mod lnurl;
mod network;
mod nostr;
mod payment;
mod permission;
mod query;
mod system;
mod transaction;
mod wallet;

pub use account::{Account, AccountPreferences, AuthIdentity};
pub use api_key::{ApiKey, ApiKeyFilter, CreateApiKeyRequest};
pub use auth::{AuthProvider, ChangePasswordRequest, SignInRequest, SignInResponse, SignUpRequest};
pub use bitcoin::{BtcAddress, BtcAddressFilter, BtcAddressType, BtcOutput, BtcOutputStatus, NewBtcAddressRequest};
pub use error::ErrorResponse;
pub use invoice::{Invoice, InvoiceFilter, InvoiceOrderBy, InvoiceStatus, LnInvoice, NewInvoiceRequest};
pub use ln_address::{LnAddress, LnAddressFilter, RegisterLnAddressRequest, UpdateLnAddressRequest};
pub use lnurl::{LNUrlpInvoiceQueryParams, LnURLPayRequest, LnUrlCallback, LnUrlSuccessAction};
pub use network::BtcNetwork;
pub use nostr::{NostrNIP05QueryParams, NostrNIP05Response};
pub use payment::{BtcPayment, InternalPayment, LnPayment, Payment, PaymentFilter, PaymentStatus, SendPaymentRequest};
pub use permission::Permission;
pub use query::OrderDirection;
pub use system::{HealthCheck, HealthStatus, SetupInfo, VersionInfo};
pub use transaction::{Currency, Ledger};
pub use wallet::{Asset, Balance, Contact, CreateWalletRequest, Wallet, WalletFilter, WalletOverview};

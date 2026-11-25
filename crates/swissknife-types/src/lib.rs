//! Shared API/contract types for Numeraire SwissKnife.
//!
//! These are pure data — entities, shared enums, and request/response types —
//! and are the single source of truth for the backend, the integration tests,
//! and external clients/SDKs. They carry their own serde + `utoipa::ToSchema`
//! annotations so the wire shape lives with the type; behaviour stays in the
//! app's use cases.

mod account;
mod agent;
mod spending_policy;
pub use spending_policy::*;
mod api_key;
pub use agent::*;
mod auth;
mod bitcoin;
mod error;
mod event;
mod invoice;
mod ln_address;
mod lnurl;
mod network;
mod nostr;
mod notification;
mod offer;
mod payment;
mod permission;
mod query;
mod system;
mod transaction;
mod wallet;

pub use account::{
    Account, AccountFilter, AccountPreferences, AuthIdentity, CreateAccountRequest, PatchAccountPreferencesRequest,
    UpdateAccountPermissionsRequest, UpdateAccountPreferencesRequest, UpdateAccountRequest,
};
pub use api_key::{ApiKey, ApiKeyFilter, CreateApiKeyRequest};
pub use auth::{
    AuthProvider, ChangePasswordRequest, CreateLocalLoginRequest, LocalLogin, LocalLoginReset,
    ResetLocalPasswordRequest, SignInRequest, SignInResponse, SignUpRequest, UpdateLocalLoginRequest,
};
pub use bitcoin::{BtcAddress, BtcAddressFilter, BtcAddressType, BtcOutput, BtcOutputStatus, NewBtcAddressRequest};
pub use error::{ErrorCode, ErrorResponse};
pub use event::{ClientEvent, ClientEventStreamQuery, ClientEventType};
pub use invoice::{
    Invoice, InvoiceFilter, InvoiceOrderBy, InvoiceStatus, LnInvoice, LnUrlInvoiceData, MerchantRevenue,
    MerchantRevenueQuery, NewInvoiceRequest,
};
pub use ln_address::{LnAddress, LnAddressFilter, RegisterLnAddressRequest, UpdateLnAddressRequest};
pub use lnurl::{
    LNUrlpInvoiceQueryParams, LnURLPayRequest, LnUrlCallback, LnUrlPaySuccessAction, LnUrlSuccessAction,
    LnUrlVerifyResponse,
};
pub use network::BtcNetwork;
pub use nostr::{NostrNIP05QueryParams, NostrNIP05Response};
pub use notification::*;
pub use offer::*;
pub use payment::{
    BtcPayment, InternalPayment, LnPayment, Payment, PaymentExecutionState, PaymentFeeEstimate, PaymentFilter,
    PaymentStatus, PaymentSubmission, SendPaymentRequest, SubmitPaymentRequest, WithdrawalSubmission,
};
pub use permission::Permission;
pub use query::OrderDirection;
pub use system::{HealthCheck, HealthStatus, SetupInfo, VersionInfo};
pub use transaction::{Currency, Ledger};
pub use wallet::{
    Asset, Balance, Contact, CreateWalletRequest, Protocol, Wallet, WalletFilter, WalletOverview, NATIVE_ASSET_REF,
};

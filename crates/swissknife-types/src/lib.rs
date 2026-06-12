//! Shared API/contract types for Numeraire SwissKnife.
//!
//! These are pure data — entities, shared enums, and request/response types —
//! and are the single source of truth for the backend, the integration tests,
//! and external clients/SDKs. They carry their own serde + `utoipa::ToSchema`
//! annotations so the wire shape lives with the type; behaviour stays in the
//! app's use cases.

mod api_key;
mod bitcoin;
mod invoice;
mod ln_address;
mod lnurl;
mod network;
mod payment;
mod permission;
mod request;
mod response;
mod transaction;
mod wallet;

pub use api_key::ApiKey;
pub use bitcoin::{BtcAddress, BtcAddressType, BtcOutput, BtcOutputStatus};
pub use invoice::{Invoice, InvoiceStatus, LnInvoice};
pub use ln_address::LnAddress;
pub use lnurl::{LnUrlCallback, LnUrlSuccessAction};
pub use network::BtcNetwork;
pub use payment::{BtcPayment, InternalPayment, LnPayment, Payment, PaymentStatus};
pub use permission::Permission;
pub use request::{
    CreateApiKeyRequest, LNUrlpInvoiceQueryParams, NewBtcAddressRequest, NewInvoiceRequest, NostrNIP05QueryParams,
    RegisterLnAddressRequest, RegisterWalletRequest, SendPaymentRequest, SignInRequest, SignUpRequest,
    UpdateLnAddressRequest,
};
pub use response::{ErrorResponse, NostrNIP05Response, SignInResponse};
pub use transaction::{Currency, Ledger};
pub use wallet::{Balance, Contact, Wallet, WalletOverview};

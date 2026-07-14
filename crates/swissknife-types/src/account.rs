use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_with::{serde_as, DisplayFromStr};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{AuthProvider, OrderDirection, Permission, Wallet};

/// An account is the owner and authorization boundary for identities, wallets,
/// API keys, permissions, and account-scoped preferences.
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct Account {
    /// Stable internal account ID.
    pub id: Uuid,

    /// Optional human-readable name for the account.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Numeraire")]
    pub display_name: Option<String>,

    /// Login identity currently linked to this account, when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<AuthIdentity>,

    /// Permissions stored for this account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<Permission>>,

    /// Account-scoped dashboard and UI preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<AccountPreferences>,

    /// Wallets owned by this account.
    ///
    /// These include asset metadata, balances, and the linked Lightning
    /// Address, but not payments, invoices, Bitcoin addresses, or contacts.
    /// Fetch a wallet by ID when those related resources are needed.
    pub wallets: Vec<Wallet>,

    /// Date of creation in database.
    pub created_at: DateTime<Utc>,

    /// Date of update in database.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// A login identity from an authentication provider linked to an account.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct AuthIdentity {
    /// Stable internal identity ID.
    pub id: Uuid,

    /// Authentication provider namespace, such as `jwt` or `oauth2`.
    pub provider: AuthProvider,

    /// Provider subject, such as a JWT username or OAuth2 `sub`.
    #[schema(example = "auth0|numeraire")]
    pub subject: String,

    /// Date of creation in database.
    pub created_at: DateTime<Utc>,
}

/// Account-scoped dashboard and UI preferences.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct AccountPreferences {
    /// Versioned dashboard settings document stored by the server.
    #[schema(example = json!({ "theme": "system" }))]
    pub dashboard_settings: Value,

    /// Date of creation in database.
    pub created_at: DateTime<Utc>,

    /// Date of update in database.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Replace account-scoped dashboard preferences.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateAccountPreferencesRequest {
    /// Versioned dashboard settings document stored by the server.
    #[schema(example = json!({ "theme": "system" }))]
    pub dashboard_settings: Value,
}

/// Create an account.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateAccountRequest {
    /// Optional human-readable name for the account.
    #[schema(example = "Numeraire")]
    pub display_name: Option<String>,

    /// Initial permissions stored for the account.
    #[serde(default)]
    pub permissions: Vec<Permission>,
}

/// Replace the editable account profile fields.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateAccountRequest {
    /// Human-readable name for the account. Use `null` to clear it.
    #[schema(example = "Numeraire")]
    pub display_name: Option<String>,
}

/// Replace permissions stored for an account.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateAccountPermissionsRequest {
    /// Complete permission set to persist for the account.
    pub permissions: Vec<Permission>,
}

/// Account query filter.
#[serde_as]
#[derive(Clone, Debug, Default, Deserialize, Serialize, IntoParams)]
pub struct AccountFilter {
    /// Total amount of results to return.
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<u64>,

    /// Offset where to start returning results.
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<u64>,

    /// Account IDs to include.
    pub ids: Option<Vec<Uuid>>,

    /// Direction of the ordering by creation date.
    #[serde(default)]
    pub order_direction: OrderDirection,
}

impl Default for AccountPreferences {
    fn default() -> Self {
        Self {
            dashboard_settings: json!({}),
            created_at: DateTime::<Utc>::default(),
            updated_at: None,
        }
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{AuthProvider, Permission};

/// An account is the owner and authorization boundary for identities, wallets,
/// API keys, permissions, and account-scoped preferences.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct Account {
    /// Stable internal account ID.
    pub id: Uuid,

    /// Optional human-readable name for the account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Login identity currently linked to this account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<AuthIdentity>,

    /// Permissions stored for this account.
    ///
    /// These are authoritative for local JWT identities. OAuth2 requests use
    /// token claims as the effective permissions instead of mirroring claims
    /// into this field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<Permission>>,

    /// Account-scoped dashboard and UI preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<AccountPreferences>,

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
    pub subject: String,

    /// Date of creation in database.
    pub created_at: DateTime<Utc>,

    /// Date of update in database.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
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

impl Default for AccountPreferences {
    fn default() -> Self {
        Self {
            dashboard_settings: json!({}),
            created_at: DateTime::<Utc>::default(),
            updated_at: None,
        }
    }
}

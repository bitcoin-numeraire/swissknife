use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::Permission;

/// An account is the owner and authorization boundary for identities, wallets,
/// API keys, permissions, and account-scoped preferences.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct Account {
    /// Stable internal account ID.
    #[schema(example = "018f3d1d-6b19-7c81-b7c1-2a0f46b9a331")]
    pub id: Uuid,

    /// Optional human-readable name for the account.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Numeraire")]
    pub display_name: Option<String>,

    /// Login identities currently linked to this account.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = json!([
        {
            "id": "018f3d1d-6b19-7c81-b7c1-2a0f46b9a332",
            "provider": "oauth2",
            "subject": "auth0|numeraire",
            "created_at": "2026-07-06T12:00:00Z"
        }
    ]))]
    pub identities: Option<Vec<AuthIdentity>>,

    /// Permissions stored for this account.
    ///
    /// These are authoritative for local JWT identities. OAuth2 requests use
    /// token claims as the effective permissions instead of mirroring claims
    /// into this field.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(["read:wallet", "write:wallet"]))]
    pub permissions: Option<Vec<Permission>>,

    /// Account-scoped dashboard and UI preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<AccountPreferences>,

    /// Date of creation in database.
    #[schema(example = "2026-07-06T12:00:00Z")]
    pub created_at: DateTime<Utc>,

    /// Date of update in database.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "2026-07-06T12:30:00Z")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// A login identity from an authentication provider linked to an account.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct AuthIdentity {
    /// Stable internal identity ID.
    #[schema(example = "018f3d1d-6b19-7c81-b7c1-2a0f46b9a332")]
    pub id: Uuid,

    /// Authentication provider namespace, such as `jwt` or `oauth2`.
    #[schema(example = "oauth2")]
    pub provider: String,

    /// Provider subject, such as a JWT username or OAuth2 `sub`.
    #[schema(example = "auth0|numeraire")]
    pub subject: String,

    /// Date of creation in database.
    #[schema(example = "2026-07-06T12:00:00Z")]
    pub created_at: DateTime<Utc>,

    /// Date of update in database.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "2026-07-06T12:30:00Z")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Account-scoped dashboard and UI preferences.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, ToSchema)]
pub struct AccountPreferences {
    /// Versioned dashboard settings document stored by the server.
    #[schema(example = json!({ "theme": "system" }))]
    pub dashboard_settings: Value,

    /// Date of creation in database.
    #[schema(example = "2026-07-06T12:00:00Z")]
    pub created_at: DateTime<Utc>,

    /// Date of update in database.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "2026-07-06T12:30:00Z")]
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

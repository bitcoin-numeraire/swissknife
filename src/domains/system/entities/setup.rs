use serde::Serialize;
use utoipa::ToSchema;

/// App setup info
#[derive(Debug, Serialize, ToSchema)]
pub struct SetupInfo {
    /// Whether the welcome flow has been completed
    pub welcome_complete: bool,
    /// Whether the admin user has been created
    pub sign_up_complete: bool,
}

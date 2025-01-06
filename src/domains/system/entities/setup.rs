use serde::Serialize;
use utoipa::ToSchema;

/// App setup info
#[derive(Debug, Serialize, ToSchema)]
pub struct SetupInfo {
    /// Whether the welcome flow has been completed
    pub welcome_complete: bool,
    /// Whether the app is setup
    pub setup_complete: bool,
}

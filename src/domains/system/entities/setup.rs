use serde::Serialize;
use utoipa::ToSchema;

/// App setup info
#[derive(Debug, Serialize, ToSchema)]
pub struct SetupInfo {
    /// Whether the app is setup
    pub complete: bool,
}

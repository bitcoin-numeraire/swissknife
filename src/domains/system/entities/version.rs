use serde::Serialize;
use utoipa::ToSchema;

/// App version info
#[derive(Debug, Serialize, ToSchema)]
pub struct VersionInfo {
    /// Current version of the software
    #[schema(example = "0.0.1")]
    pub version: String,

    /// Build time of the software
    #[schema(example = "2024-07-03T18:13:09.093289+00:00")]
    pub build_time: String,
}

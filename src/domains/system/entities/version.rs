use serde::Serialize;

#[derive(Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub build_time: String,
}

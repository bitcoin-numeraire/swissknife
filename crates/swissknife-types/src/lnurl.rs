use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug)]
pub struct LnUrlCallback {
    pub pr: String,
    pub success_action: Option<LnUrlSuccessAction>,
    pub disposable: Option<bool>,
    pub routes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct LnUrlSuccessAction {
    /// Action type. One of `url` or `message`
    pub tag: String,

    /// Message displayed to the user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// URL for the user to open on success
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

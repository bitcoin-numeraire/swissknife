use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Default, ToSchema)]
pub struct Contact {
    /// Lightning Address
    #[schema(example = "dario_nakamoto@numeraire.tech")]
    pub ln_address: String,
}

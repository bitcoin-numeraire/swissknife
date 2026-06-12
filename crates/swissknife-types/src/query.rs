use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Direction of result ordering for list endpoints.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default, ToSchema)]
pub enum OrderDirection {
    Asc,
    #[default]
    Desc,
}

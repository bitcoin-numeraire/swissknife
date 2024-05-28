use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct PaginationFilter {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

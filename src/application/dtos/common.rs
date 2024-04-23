use serde::Deserialize;

#[derive(Deserialize)]
pub struct PaginationQueryParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

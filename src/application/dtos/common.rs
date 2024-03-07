use serde::Deserialize;

#[derive(Deserialize)]
pub struct PaginationQueryParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

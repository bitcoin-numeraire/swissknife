use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct PaginationFilter {
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub limit: Option<u64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub offset: Option<u64>,
}

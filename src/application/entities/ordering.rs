use sea_orm::Order;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

#[derive(
    Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default, ToSchema,
)]
pub enum OrderDirection {
    Asc,
    #[default]
    Desc,
}

impl From<OrderDirection> for Order {
    fn from(val: OrderDirection) -> Self {
        match val {
            OrderDirection::Asc => Order::Asc,
            OrderDirection::Desc => Order::Desc,
        }
    }
}

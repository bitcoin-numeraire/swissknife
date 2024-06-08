use sea_orm::Order;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Clone, Debug, EnumString, Deserialize, Serialize, Display, PartialEq, Eq, Default)]
pub enum OrderDirection {
    #[default]
    Desc,
    Asc,
}

impl Into<Order> for OrderDirection {
    fn into(self) -> Order {
        match self {
            OrderDirection::Asc => Order::Asc,
            OrderDirection::Desc => Order::Desc,
        }
    }
}

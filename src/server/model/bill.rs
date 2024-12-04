use serde::{Deserialize, Serialize};
use crate::server::model::item::Item;

#[derive(Debug, Serialize)]
pub(crate) struct GetBillResponse {
    pub bill: Option<Bill>,
}

/// A bill that binds to a table, and binds to zero to many bill items
#[derive(Debug, Serialize)]
pub(crate) struct Bill {
    pub id: i64,
    pub items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PostBillItemsRequest {
    pub items: Vec<MenuItemId>,
}

type MenuItemId = i32;

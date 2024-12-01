use serde::{Deserialize, Serialize};
use crate::server::model::item::Item;

#[derive(Debug, Serialize)]
pub(crate) struct GetBillResponse {
    pub bill: Option<Bill>,
}

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

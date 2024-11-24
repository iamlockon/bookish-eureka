use serde::{Deserialize, Serialize};
use crate::server::model::item::Item;

#[derive(Debug, Serialize)]
pub(crate) struct GetBillResponse {
    pub result_code: Option<String>,
    pub bill: Option<Bill>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Bill {
    pub id: i64,
    pub items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PostBillItemsRequest {
    pub items: Vec<ItemModification>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PostBillItemsResponse {
    pub result_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteBillItemsRequest {
    pub items: Vec<i32>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DeleteBillItemsResponse {
    pub result_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ItemModification {
    pub id: i32,
    pub count: i16,
}

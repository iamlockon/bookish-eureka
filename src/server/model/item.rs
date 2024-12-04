use serde::Serialize;

/// A bill item that contains the order details
#[derive(Debug, Serialize)]
pub(crate) struct Item {
    /// menu item id
    pub id: i64,
    /// menu item name
    pub name: String,
    /// time to deliver to customers
    pub time_to_deliver: i32,
    /// status of the order
    pub state: String,
}

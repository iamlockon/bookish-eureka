use serde::Serialize;
use crate::server::model::item::Item;

#[derive(Debug, Default, Serialize)]
pub(crate) struct Order {
    pub items: Vec<Item>
}


use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Item {
    pub name: String,
    pub price: u32,
}
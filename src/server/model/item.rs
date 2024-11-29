use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Item {
    pub id: i64,
    pub name: String,
    pub time_to_deliver: i32,
}

use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Item {
    pub id: u32,
    pub name: String
}

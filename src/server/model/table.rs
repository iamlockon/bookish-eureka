use serde::{Deserialize, Serialize};
use crate::server::model::bill::Bill;

#[derive(Debug, Serialize)]
pub(crate) struct PatchTablesResponse {
    pub result_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PatchTablesRequest {
    pub customer_count: i16,
}

#[derive(Debug, Serialize)]
pub(crate) struct GetTablesResponse {
    pub result_code: Option<String>,
    pub tables: Option<Vec<Table>>, 
}

#[derive(Debug, Serialize)]
pub(crate) struct Table {
    pub id: u8,
    pub bill_id: Option<u64>, // only when table is occupied there will be associated bill
}
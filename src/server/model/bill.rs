use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(crate) struct GetBillsResponse {
    pub result_code: Option<String>,
    pub bills: Vec<Bill>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GetBillResponse {
    pub result_code: Option<String>,
    pub bills: Bill,
}

#[derive(Debug, Serialize)]
pub struct Bill {
    pub id: i64,
    pub table_id: i16,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub checkout_at: Option<String>,
    pub customer_count: i16,
}

#[derive(Debug, Serialize)]
pub(crate) struct PostBillsResponse {
    pub result_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PostBillsRequest {
    pub table_id: i16,
    pub customer_count: i16,
}

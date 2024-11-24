use serde::Deserialize;

pub(crate) mod bill;
pub(crate) mod config;
pub(crate) mod item;
pub(crate) mod table;

#[derive(Debug, Deserialize)]
pub(crate) struct CommonRequestParams {
    pub page: Option<u8>,
    pub page_size: Option<u8>,
}
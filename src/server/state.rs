use crate::server::database::pool::{PgPool, Pool};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AppState {
    db_read_pool: Pool,
    db_write_pool: Pool,
}

impl AppState {
    pub fn new(db_read_pool: Pool, db_write_pool: Pool) -> Self {
        Self {
            db_read_pool,
            db_write_pool,
        }
    }
    pub fn get_db_read_pool(&self) -> Pool {
        self.db_read_pool.clone()
    }

    pub fn get_db_write_pool(&self) -> Pool {
        self.db_write_pool.clone()
    }
}

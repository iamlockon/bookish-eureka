use crate::server::database::pool::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AppState {
    db_read_pool: Arc<PgPool>,
    db_write_pool: Arc<PgPool>,
}

impl AppState {
    pub fn new(db_read_pool: Arc<PgPool>, db_write_pool: Arc<PgPool>) -> Self {
        Self {
            db_read_pool,
            db_write_pool,
        }
    }
    pub fn get_db_read_pool(&self) -> Arc<PgPool> {
        self.db_read_pool.clone()
    }

    pub fn get_db_write_pool(&self) -> Arc<PgPool> {
        self.db_write_pool.clone()
    }
}

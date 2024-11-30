use tokio_postgres::Client;
use crate::server::database::pool::{DbClient, Pool};

#[derive(Clone)]
pub(crate) struct AppState {
    db_read_pool: Pool<Client>,
    db_write_pool: Pool<Client>,
}

impl AppState {
    pub fn new(db_read_pool: Pool<Client>, db_write_pool: Pool<Client>) -> Self {
        Self {
            db_read_pool,
            db_write_pool,
        }
    }
    pub fn get_db_read_pool(&self) -> Pool<Client> {
        self.db_read_pool.clone()
    }

    pub fn get_db_write_pool(&self) -> Pool<impl DbClient> {
        self.db_write_pool.clone()
    }
}

#[cfg(test)]
mod test {
    use std::any::{Any, TypeId};
    use super::*;

    #[actix_web::test]
    async fn app_state() {
        async {
            let (read_pool, write_pool) = (Pool::new().await, Pool::new().await);
            let state = AppState::new(read_pool.unwrap(), write_pool.unwrap());
            assert_eq!(state.get_db_read_pool().type_id(), TypeId::of::<Pool<DbClient>>());
            assert_eq!(state.get_db_write_pool().type_id(), TypeId::of::<Pool<DbClient>>());
        }.await;
    }
}
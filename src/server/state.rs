use tokio_postgres::Client;
#[cfg(test)]
use crate::server::database::connection::MockClient;
use crate::server::database::pool::{DbClient, Pool};

#[derive(Clone)]
#[cfg(not(test))]
pub(crate) struct AppState {
    db_read_pool: Pool<Client>,
    db_write_pool: Pool<Client>,
}

#[derive(Clone)]
#[cfg(test)]
pub(crate) struct AppState {
    db_read_pool: Pool<MockClient>,
    db_write_pool: Pool<MockClient>,
}

impl AppState {
    #[cfg(not(test))]
    pub fn new(db_read_pool: Pool<Client>, db_write_pool: Pool<Client>) -> Self {
        Self {
            db_read_pool,
            db_write_pool,
        }
    }

    #[cfg(not(test))]
    pub fn get_db_read_pool(&self) -> Pool<Client> {
        self.db_read_pool.clone()
    }
    
    #[cfg(not(test))]
    pub fn get_db_write_pool(&self) -> Pool<Client> {
        self.db_write_pool.clone()
    }

    #[cfg(test)]
    pub fn new(db_read_pool: Pool<MockClient>, db_write_pool: Pool<MockClient>) -> Self {
        Self {
            db_read_pool,
            db_write_pool,
        }
    }

    #[cfg(test)]
    pub fn get_db_read_pool(&self) -> Pool<MockClient> {
        self.db_read_pool.clone()
    }

    #[cfg(test)]
    pub fn get_db_write_pool(&self) -> Pool<MockClient> {
        self.db_write_pool.clone()
    }
}

#[cfg(test)]
mod test {
    use std::any::{Any, TypeId};
    use crate::server::database::connection::MockClient;
    use super::*;

    #[actix_web::test]
    async fn app_state() {
        async {
            let (read_pool, write_pool) = (Pool::<MockClient>::new().await, Pool::<MockClient>::new().await);
            let state = AppState::new(read_pool.unwrap(), write_pool.unwrap());
            assert_eq!(state.get_db_read_pool().type_id(), TypeId::of::<Pool<MockClient>>());
            assert_eq!(state.get_db_write_pool().type_id(), TypeId::of::<Pool<MockClient>>());
        }.await;
    }
}
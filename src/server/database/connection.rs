use tokio_postgres::Client;
use crate::server::database::pool::{Pool};

pub(crate) struct Connection {
    pub(crate) client: Option<Client>,
    pub(crate) pool: Pool,
}

impl Connection {
    pub fn new(client: Client, pool: Pool) -> Self {
        Self { client: Some(client), pool }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.pool.release(self.client.take().unwrap());
    }
}
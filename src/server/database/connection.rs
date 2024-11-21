use tokio_postgres::Client;

pub(crate) struct Connection {
    pub(crate) client: Client,
}

impl Connection {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

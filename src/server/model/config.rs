use std::net::SocketAddrV4;

/// Server configs
#[derive(Debug)]
pub(crate) struct ServerConfig {
    pub addr: SocketAddrV4,
    pub db_read_pool_conn_str: String,
    pub db_write_pool_conn_str: String,
}

impl ServerConfig {
    pub fn new(
        addr: SocketAddrV4,
        db_read_pool_conn_str: String,
        db_write_pool_conn_str: String,
    ) -> Self {
        Self {
            addr,
            db_read_pool_conn_str,
            db_write_pool_conn_str,
        }
    }
}

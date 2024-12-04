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

#[cfg(test)]
mod tests {
    use std::net::SocketAddrV4;
    use std::str::FromStr;
    use crate::server::model::config::ServerConfig;

    #[test]
    fn test_new_config() {
        const CONN_STR: &str = "host=localhost";
        const ADDR: &'static str = "0.0.0.0:8080";
        let config = ServerConfig::new(
            SocketAddrV4::from_str(ADDR).unwrap(),
            CONN_STR.to_string(),
            CONN_STR.to_string()
        );
        assert_eq!(config.addr.to_string(), ADDR);
        assert_eq!(config.db_read_pool_conn_str, CONN_STR);
        assert_eq!(config.db_write_pool_conn_str, CONN_STR);
    }
}
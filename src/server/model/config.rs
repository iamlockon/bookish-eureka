use std::net::SocketAddrV4;

/// Server configs
#[derive(Debug)]
pub(crate) struct ServerConfig {
    pub addr: SocketAddrV4
}

impl ServerConfig {
    pub fn new(addr: SocketAddrV4) -> Self {
        Self {
            addr
        }
    }
}
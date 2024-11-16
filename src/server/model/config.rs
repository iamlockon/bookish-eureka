use std::net::{Ipv4Addr, SocketAddrV4};
use std::str::FromStr;

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
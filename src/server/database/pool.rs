use crate::server::database::connection::Connection;
use anyhow::Error;
use log::{error, info, warn};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time;
use tokio_postgres::{connect, NoTls};

pub(crate) struct PgPool {
    /// pool name
    name: String,
    /// connections in the pool, accessed in a FIFO manner
    connections: Arc<Mutex<VecDeque<Connection>>>,
}

impl PgPool {
    /// create a connection pool with default configuration, which creates 10 connections.
    pub async fn new(conn_str: String) -> Result<Self, Error> {
        const DEFAULT_SIZE: usize = 10;
        let mut set = JoinSet::new();

        for _ in 0..DEFAULT_SIZE {
            let str = conn_str.clone();
            set.spawn(async move { connect(str.as_str(), NoTls).await });
        }

        let mut connections = VecDeque::with_capacity(DEFAULT_SIZE);
        while let Some(res) = set.join_next().await {
            match res {
                Ok(res) => match res {
                    Ok((client, conn)) => {
                        tokio::spawn(async move {
                            if let Err(e) = conn.await {
                                error!("connection returned error and aborted, {}", e);
                            }
                        });
                        connections.push_back(Connection::new(client));
                    }
                    Err(e) => {
                        error!("failed to connect, {}", e);
                    }
                },
                Err(e) => {
                    error!("join_next failed when joining, {}", e);
                }
            }
        }

        Ok(Self {
            name: "read".to_string(),
            connections: Arc::new(Mutex::new(connections)),
        })
    }

    /// acquire a connection after locking the mutex, which might wait indefinitely.
    pub async fn acquire(&self) -> Option<Connection> {
        let arc_connections = self.connections.clone();
        let mut connections = arc_connections.lock().await;
        if let Some(conn) = connections.pop_front() {
            return Some(conn);
        }
        None
    }

    /// acquire a connection with specified timeout, bail out if timeout exceeds.
    async fn acquire_with_timeout(&self, timeout: u64) -> Option<Connection> {
        let sleep = time::sleep(Duration::from_millis(timeout));
        tokio::pin!(sleep);
        let arc_connections = self.connections.clone();
        tokio::select! {
            mut connections = arc_connections.lock() => {
                if let Some(conn) = connections.pop_front() {
                    return Some(conn);
                }
                None
            },
            _ = &mut sleep => {
                error!("timed out to acquire a new connection from pool after {} millisecond", timeout);
                None
            },
        }
    }
}

impl Drop for PgPool {
    fn drop(&mut self) {
        let name = self.name.clone();
        match tokio::runtime::Builder::new_current_thread().build() {
            Ok(rt) => {
                rt.block_on(async {
                    let arc_connections = self.connections.clone();
                    let mut connections = arc_connections.lock().await;
                    connections.clear();
                });
                info!("cleaned up connection pool ({})", name);
            }
            Err(error) => {
                warn!("failed to clean up connection pool ({}), {}", name, error);
            }
        }
    }
}

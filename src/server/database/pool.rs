use crate::server::database::connection::Connection;
use anyhow::Error;
use log::{error, info, warn};
use std::collections::VecDeque;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time;
use tokio_postgres::{connect, Client, NoTls};


pub(crate) struct PgPool {
    /// pool name
    name: String,
    /// connections in the pool, accessed in a FIFO manner
    connections: Mutex<VecDeque<Connection>>,
}

pub(crate) struct Pool(Arc<PgPool>);

impl Clone for Pool {
    fn clone(&self) -> Pool {
        Pool(self.0.clone())
    }
}

impl Pool {
    const DEFAULT_SIZE: usize = 10;
    /// create a connection pool with default configuration
    pub async fn new() -> Result<Self, Error> {
        let shared = Arc::new(PgPool{name: "name".to_string(), connections: Mutex::new(VecDeque::with_capacity(Self::DEFAULT_SIZE))});
        let pool = Self(shared);

        Ok(pool)
    }
    
    pub async fn init(&mut self, conn_str: String) -> Result<(), Error> {

        let mut connections = VecDeque::with_capacity(Self::DEFAULT_SIZE);
        let mut set = JoinSet::new();
        for _ in 0..Self::DEFAULT_SIZE {
            let str = conn_str.clone();
            set.spawn(async move { connect(str.as_str(), NoTls).await });
        }
        while let Some(res) = set.join_next().await {
            match res {
                Ok(res) => match res {
                    Ok((client, conn)) => {
                        tokio::spawn(async move {
                            if let Err(e) = conn.await {
                                error!("connection returned error and aborted, {}", e);
                            }
                        });
                        connections.push_back(Connection::new(client, self.clone()));
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
        self.0.connections.lock().await.append(&mut connections);
        Ok(())
    }

    /// acquire a connection after locking the mutex, which might wait indefinitely.
    pub async fn acquire(&self) -> Option<Connection> {
        let mut connections = self.0.connections.lock().await;
        if let Some(conn) = connections.pop_front() {
            return Some(conn);
        }
        None
    }

    /// acquire a connection with specified timeout, bail out if timeout exceeds.
    async fn acquire_with_timeout(&self, timeout: u64) -> Option<Connection> {
        let sleep = time::sleep(Duration::from_millis(timeout));
        tokio::pin!(sleep);
        tokio::select! {
            mut connections = self.0.connections.lock() => {
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
    
    pub fn release(&mut self, client: Client) {
        let pool = self.0.clone();
        thread::spawn(move || {
            let mut connections = pool.connections.blocking_lock();
            connections.push_back(Connection::new(client, Pool(pool.clone())));
        });
    }
}

impl Drop for PgPool {
    fn drop(&mut self) {
        let name = self.name.clone();
        match tokio::runtime::Builder::new_current_thread().build() {
            Ok(rt) => {
                rt.block_on(async {
                    let mut connections = self.connections.lock().await;
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

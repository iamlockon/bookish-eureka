use crate::server::database::connection::{Connection, WrappedClient};
use anyhow::Error;
use log::{error, info};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, thread};
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time;
use tokio_postgres::{Row, ToStatement};
use tokio_postgres::row::RowIndex;
use tokio_postgres::types::{FromSql, ToSql, Type};
use crate::server::controller::error::CustomError;

#[derive(Default)]
pub(crate) struct PgPool<M>
where M: DbClient
{
    /// pool name
    name: String,
    /// connections in the pool, accessed in a FIFO manner
    connections: Mutex<VecDeque<M>>,
}

impl<M> PgPool<M>
where M: DbClient + Send
{
    pub fn new(name: String) -> Self {
        Self {
            name,
            connections: Mutex::new(VecDeque::new()),
        }
    }
}

/// for test
pub(crate) struct MockClient;
impl DbClient for MockClient {
    #[allow(unused_variables)]
    async fn query<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<impl GenericRow>, tokio_postgres::Error>
    where
        T: ?Sized + ToStatement
    {
        println!("query mock client");
        Ok(Vec::<MockRow>::new())
    }

    #[allow(unused_variables)]
    async fn execute<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, tokio_postgres::Error>
    where
        T: ?Sized + ToStatement
    {
        println!("execute mock client");
        Ok(u64::MIN)
    }

    async fn transaction(&mut self) -> Result<WrappedTransaction<impl GenericTransaction>, tokio_postgres::Error> {
        Ok(WrappedTransaction(MockTransaction{}))
    }
}

pub(crate) trait DbClient: Send {
    async fn query<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<impl GenericRow>, tokio_postgres::Error>
    where
        T: ?Sized + ToStatement;
    async fn execute<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<u64, tokio_postgres::Error>
    where
        T: ?Sized + ToStatement;
    async fn transaction(&mut self) -> Result<WrappedTransaction<impl GenericTransaction>, tokio_postgres::Error>;
}

pub(crate) struct CommonPool<M>
where M : DbClient + Send + 'static
{
    pub name: String,
    pub connections: Mutex<VecDeque<Connection<M>>>,
}

pub(crate) struct Pool<M>(Arc<CommonPool<M>>) where M : DbClient + Send + 'static;

impl<M> Clone for Pool<M>
where M : DbClient + Send
{
    fn clone(&self) -> Pool<M> {
        Pool(self.0.clone())
    }
}

pub(crate) struct WrappedTransaction<T: GenericTransaction>(pub T);

impl<T: GenericTransaction> Deref for WrappedTransaction<T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: GenericTransaction> DerefMut for WrappedTransaction<T>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub(crate) trait GenericTransaction {
    async fn query_one<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<impl GenericRow, tokio_postgres::Error>
    where
        T: ?Sized + ToStatement;

    async fn execute<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<u64, tokio_postgres::Error>
    where
        T: ?Sized + ToStatement;

    async fn commit(self) -> Result<(), tokio_postgres::Error>;
}

pub(crate) struct WrappedRow<R: GenericRow>(R);

pub(crate) trait GenericRow {
    fn get<'a, I, T>(&'a self, idx: I) -> T
    where
        I: RowIndex + fmt::Display,
        T: FromSql<'a>;
    
    fn try_get<'a, I, T>(&'a self, idx: I) -> Result<T, CustomError>
    where
        I: RowIndex + fmt::Display,
        T: FromSql<'a>;
}

impl GenericRow for Row {
    fn get<'a, I, T>(&'a self, idx: I) -> T
    where
        I: RowIndex + Display,
        T: FromSql<'a>
    {
        self.get(idx)
    }

    fn try_get<'a, I, T>(&'a self, idx: I) -> Result<T, CustomError>
    where
        I: RowIndex + Display,
        T: FromSql<'a>
    {
        self.try_get(idx).map_err(|e| CustomError::DbError(e))
    }
}

/// for test
pub(crate) struct MockRow;
impl GenericRow for MockRow {
    #[allow(unused_variables)]
    fn get<'a, I, T>(&'a self, idx: I) -> T
    where
        I: RowIndex + Display,
        T: FromSql<'a>
    {
        println!("get for mock row");
        T::from_sql(&Type::ANY, &[0]).unwrap() // random impl
    }

    #[allow(unused_variables)]
    fn try_get<'a, I, T>(&'a self, idx: I) -> Result<T, CustomError>
    where
        I: RowIndex + Display,
        T: FromSql<'a>
    {
        println!("try_get for mock row");
        match T::from_sql(&Type::ANY, &[0]) {
            Ok(T) => Ok(T),
            Err(_) => Err(CustomError::Unknown), // random impl
        }
    }
}

/// for test
pub(crate) struct MockTransaction;
impl GenericTransaction for MockTransaction {
    #[allow(unused_variables)]
    async fn query_one<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<impl GenericRow, tokio_postgres::Error>
    where
        T: ?Sized + ToStatement
    {
        println!("query_one mock transaction");
        Ok(MockRow{})
    }

    #[allow(unused_variables)]
    async fn execute<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, tokio_postgres::Error>
    where
        T: ?Sized + ToStatement
    {
        Ok(u64::MIN)
    }

    async fn commit(self) -> Result<(), tokio_postgres::Error> {
        Ok(())
    }
}

pub(crate) mod connect_util {
    use anyhow::Context;use log::error;
    use tokio_postgres::NoTls;
    use crate::server::database::connection::WrappedClient;
    use crate::server::database::pool::{DbClient};
    #[cfg(not(test))]
    pub async fn connect(str: &str) -> WrappedClient<impl DbClient> {
        // abort the process if failed to connect db
        let (client, conn) = tokio_postgres::connect(str, NoTls).await.context("failed to create connection").unwrap();
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                error!("connection returned error and aborted, {}", e);
                // TODO: publish metrics for monitoring
            }
        });
        WrappedClient(client)
    }

    #[cfg(test)]
    pub async fn connect(str: &str) -> WrappedClient<impl DbClient> {
        let client = MockClient{};
        WrappedClient(client)
    }
}

impl<M> Pool<M>
where M : DbClient + Send + 'static
{
    const DEFAULT_SIZE: usize = 10;
    /// create a connection pool with default configuration
    pub async fn new() -> Result<Self, Error> {
        let shared = Arc::new(CommonPool{
            name: "name".to_string(),
            connections: Mutex::new(VecDeque::with_capacity(Self::DEFAULT_SIZE))
        });
        let pool = Self(shared);

        Ok(pool)
    }
    
    pub async fn init(&mut self, conn_str: String) -> Result<(), Error> {
        let mut connections: VecDeque<Connection<M>> = VecDeque::with_capacity(Self::DEFAULT_SIZE);
        let mut set = JoinSet::new();
        for _ in 0..Self::DEFAULT_SIZE {
            let str = conn_str.clone();
            set.spawn(async move { connect_util::connect(str.as_str()).await });
        }
        while let Some(res) = set.join_next().await {
            match res {
                Ok(wrapped_client) => {
                    info!("connection created");
                    connections.push_back(Connection::new(wrapped_client, self.clone()));
                },
                Err(e) => {
                    error!("join_next failed when joining, {}", e);
                }
            };
        }
        self.0.connections.lock().await.append(&mut connections);
        Ok(())
    }

    /// acquire a connection after locking the mutex, which might wait indefinitely.
    pub async fn acquire(&self) -> Option<Connection<M>> {
        let mut connections = self.0.connections.lock().await;
        if let Some(conn) = connections.pop_front() {
            return Some(conn);
        }
        None
    }

    /// acquire a connection with specified timeout, bail out if timeout exceeds.
    async fn acquire_with_timeout(&self, timeout: u64) -> Option<Connection<M>> {
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
    
    pub fn release(&mut self, client: WrappedClient<M>) {
        let pool = self.0.clone();
        let handle = thread::spawn(move || {
            let mut connections = pool.connections.blocking_lock();
            connections.push_back(Connection::new(client, Pool(pool.clone())));
        });
        handle.join().unwrap();
    }
}

// impl Drop for PgPool {
//     fn drop(&mut self) {
//         let pool = mem::take(self);
//         let handle = thread::spawn(move || {
//             info!("dropping connections");
//             println!("dropping connections");
//             let mut connections = pool.connections.blocking_lock();
//             connections.clear();
//             info!("dropped connections");
//         });
//         handle.join().unwrap();
//         info!("finished up");
//         // match tokio::runtime::Builder::new_current_thread().build() {
//         //     Ok(rt) => {
//         //         rt.block_on(async {
//         //             let mut connections = self.connections.lock().await;
//         //             connections.clear();
//         //         });
//         //         info!("cleaned up connection pool ({})", name);
//         //     }
//         //     Err(error) => {
//         //         warn!("failed to clean up connection pool ({}), {}", name, error);
//         //     }
//         // }
//     }
// }

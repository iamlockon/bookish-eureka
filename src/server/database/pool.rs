use crate::server::database::connection::{Connection};
#[cfg(test)]
use crate::server::database::connection::MockClient;
use anyhow::{anyhow, Error};
use log::{error, info, warn};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use std::{mem, thread};
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time;
use tokio_postgres::{Client, Row, ToStatement, Transaction};
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

pub(crate) trait DbClient: Send + Sync + 'static {
    type Client: Send + 'static;
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
    #[cfg(not(test))]
    async fn transaction(&mut self) -> Result<WrappedTransaction<Transaction<'_>>, tokio_postgres::Error>;
    #[cfg(test)]
    async fn transaction(&mut self) -> Result<MockTransaction, tokio_postgres::Error>;
}

pub(crate) struct CommonPool<M>
where M : DbClient<Client = M>
{
    pub connections: Mutex<VecDeque<Connection<M>>>,
}

pub(crate) struct Pool<M>(Arc<CommonPool<M>>) where M : DbClient<Client = M>;

impl<M> Clone for Pool<M>
where M : DbClient<Client = M>
{
    fn clone(&self) -> Pool<M> {
        Pool(self.0.clone())
    }
}

#[cfg(not(test))]
pub(crate) struct WrappedTransaction<T: GenericTransaction<Row>>(pub T);
#[cfg(test)]
pub(crate) struct WrappedTransaction<T: GenericTransaction<MockRow>>(pub T);

pub(crate) trait GenericTransaction<R: GenericRow> {
    async fn query_one<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<WrappedRow<R>, tokio_postgres::Error>
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

pub(crate) struct WrappedRow<R: GenericRow>(pub R);

pub(crate) trait GenericRow {
    fn get<'a, I, T>(&'a self, idx: I) -> T
    where
        I: RowIndex + Display,
        T: FromSql<'a>;
    
    fn try_get<'a, I, T>(&'a self, idx: I) -> Result<T, Error>
    where
        I: RowIndex + Display,
        T: FromSql<'a>;
}

#[cfg(not(test))]
impl GenericRow for WrappedRow<Row> {
    fn get<'a, I, T>(&'a self, idx: I) -> T
    where
        I: RowIndex + Display,
        T: FromSql<'a>
    {
        self.0.get(idx)
    }

    fn try_get<'a, I, T>(&'a self, idx: I) -> Result<T, Error>
    where
        I: RowIndex + Display,
        T: FromSql<'a>
    {
        self.0.try_get(idx).map_err(|e| anyhow!(e))
    }
}

#[cfg(not(test))]
impl GenericRow for Row {
    fn get<'a, I, T>(&'a self, idx: I) -> T
    where
        I: RowIndex + Display,
        T: FromSql<'a>
    {
        self.get(idx)
    }

    fn try_get<'a, I, T>(&'a self, idx: I) -> Result<T, Error>
    where
        I: RowIndex + Display,
        T: FromSql<'a>
    {
        self.try_get(idx).map_err(|e| anyhow!(e))
    }
}

/// for test
#[cfg(test)]
pub(crate) struct MockRow;

#[cfg(test)]
impl MockRow {
    pub fn new() -> Self {
        Self{}
    }
}

#[cfg(test)]
impl GenericRow for WrappedRow<MockRow> {
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
    fn try_get<'a, I, T>(&'a self, idx: I) -> Result<T, Error>
    where
        I: RowIndex + Display,
        T: FromSql<'a>
    {
        println!("try_get for mock row");
        match T::from_sql(&Type::ANY, &[0]) {
            Ok(t) => Ok(t),
            Err(e) => Err(anyhow!(e)), // random impl
        }
    }
}

#[cfg(test)]
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
    fn try_get<'a, I, T>(&'a self, idx: I) -> Result<T, Error>
    where
        I: RowIndex + Display,
        T: FromSql<'a>
    {
        println!("try_get for mock row");
        match T::from_sql(&Type::ANY, &[0]) {
            Ok(t) => Ok(t),
            Err(e) => Err(anyhow!(e)), // random impl
        }
    }
}

/// for test
#[cfg(test)]
pub(crate) struct MockTransaction;
#[cfg(test)]
impl MockTransaction {
    pub fn new() -> Self {
        Self{}
    }
}

#[cfg(test)]
impl GenericTransaction<MockRow> for MockTransaction {
    #[allow(unused_variables)]
    async fn query_one<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<WrappedRow<MockRow>, tokio_postgres::Error>
    where
        T: ?Sized + ToStatement
    {
        println!("query_one mock transaction");
        Ok(WrappedRow(MockRow::new()))
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
    use anyhow::Context;
    use log::error;
    use tokio_postgres::{Client, NoTls};
    use crate::server::database::pool::{DbClient};
    
    #[cfg(not(test))]
    pub async fn connect<M: DbClient + From<Client>>(str: &str) -> M {
        // abort the process if failed to connect db
        let (client, conn) = tokio_postgres::connect(str, NoTls).await.context("failed to create connection").unwrap();
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                error!("connection returned error and aborted, {}", e);
                // TODO: publish metrics for monitoring
            }
        });
        client.into()
    }

    #[cfg(test)]
    use crate::server::database::pool::MockClient;
    #[cfg(test)]
    pub async fn connect<M: DbClient + From<MockClient>>(_: &str) -> M {
        let client = MockClient{};
        client.into()
    }
}

pub(crate) trait Init {
    async fn init(&mut self, conn_str: String) -> Result<(), Error>;
}

#[cfg(not(test))]
impl Init for Pool<Client> {
    #[cfg(not(test))]
    async fn init(&mut self, conn_str: String) -> Result<(), Error> {
        let mut connections: VecDeque<Connection<Client>> = VecDeque::with_capacity(Self::DEFAULT_SIZE);
        let mut set = JoinSet::new();
        for _ in 0..Self::DEFAULT_SIZE {
            let str = conn_str.clone();
            set.spawn(async move { connect_util::connect::<Client>(str.as_str()).await });
        }
        while let Some(res) = set.join_next().await {
            match res {
                Ok(wrapped_client) => {
                    info!("connection created");
                    connections.push_back(Connection::<Client>::new(wrapped_client, self.clone()));
                },
                Err(e) => {
                    error!("join_next failed when joining, {}", e);
                }
            };
        }
        self.0.connections.lock().await.append(&mut connections);
        Ok(())
    }
}

#[cfg(test)]
impl Init for Pool<MockClient> {
    async fn init(&mut self, _: String) -> Result<(), Error> {
        println!("initializing MockClient.");
        self.0.connections.lock().await.push_back(Connection::new(MockClient{}, self.clone()));
        Ok(())
    }
}

impl<M> Pool<M>
where M : DbClient<Client = M>
{
    const DEFAULT_SIZE: usize = 10;
    /// create a connection pool with default configuration
    pub async fn new() -> Result<Self, Error> {
        let shared = Arc::new(CommonPool{
            connections: Mutex::new(VecDeque::with_capacity(Self::DEFAULT_SIZE))
        });
        let pool = Self(shared);
        Ok(pool)
    }

    /// acquire a connection with specified timeout, bail out if timeout exceeds.
    #[allow(unused)]
    pub async fn acquire(&self, timeout: u64) -> Option<Connection<M>> {
        let sleep = time::sleep(Duration::new(timeout, 0));
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
    
    pub fn release(&mut self, client: M::Client) {
        let pool = self.0.clone();
        let handle = thread::spawn(move || {
            let mut connections = pool.connections.blocking_lock();
            connections.push_back(Connection::new(client, Pool(pool.clone())));
        });
        handle.join().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use crate::server::DB_TIMEOUT_SECONDS;
    use super::*;

    #[tokio::test]
    async fn test_new() {
        let pool = Pool::<MockClient>::new().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_acquire_and_release() {
        let mut pool = Pool::<MockClient>::new().await.unwrap();
        assert!(pool.acquire(DB_TIMEOUT_SECONDS).await.is_none());

        pool.init("conn_str".to_string()).await.unwrap();
        {
            let _conn = match pool.acquire(DB_TIMEOUT_SECONDS).await {
                Some(conn) => conn,
                None => panic!("should get some"),
            };
            assert!(pool.acquire(DB_TIMEOUT_SECONDS).await.is_none());
        } // conn drops here, and is released automatically

        assert!(pool.acquire(DB_TIMEOUT_SECONDS).await.is_some());
        assert!(pool.acquire(DB_TIMEOUT_SECONDS).await.is_some());
    }
}

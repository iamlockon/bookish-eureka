use tokio_postgres::{Client, Error, ToStatement, Transaction};
use tokio_postgres::types::ToSql;
use crate::server::database::pool::{Pool, DbClient, GenericTransaction, WrappedTransaction, GenericRow};

pub(crate) struct Connection<M>
where M: DbClient + Send + 'static,
{
    pub(crate) client: Option<WrappedClient<M>>,
    pub(crate) pool: Pool<M>,
}

pub(crate) struct WrappedClient<C>(pub C)
where C: DbClient;

// impl<C: ClientImpl> WrappedClient<C> {
//     pub fn
// }

impl<C> DbClient for WrappedClient<C>
where C: DbClient + Send
{
    async fn query<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<impl GenericRow>, Error>
    where
        T: ?Sized + ToStatement
    {
        self.0.query(statement, params).await
    }

    async fn execute<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement
    {
        self.0.execute(statement, params).await
    }

    async fn transaction(&mut self) -> Result<WrappedTransaction<impl GenericTransaction>, Error>
    {
        self.0.transaction().await
    }
}

impl DbClient for Client {
    async fn query<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<impl GenericRow>, Error>
    where
        T: ?Sized + ToStatement
    {
        self.query(statement, params).await
    }

    async fn execute<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement,
    {
        self.execute(statement, params).await
    }

    async fn transaction(&mut self) -> Result<WrappedTransaction<impl GenericTransaction>, Error>
    {
        let txn = self.transaction().await;
        match txn {
            Ok(txn) => Ok(WrappedTransaction(txn)),
            Err(e) => Err(e),
        }
    }
}

impl GenericTransaction for Transaction<'_> {
    async fn query_one<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<impl GenericRow, Error>
    where
        T: ?Sized + ToStatement
    {
        self.query_one(statement, params).await
    }

    async fn execute<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement
    {
        self.execute(statement, params).await
    }

    async fn commit(self) -> Result<(), Error> {
        self.commit().await
    }
}

impl<M> Connection<M>
where M : DbClient + Send
{
    pub fn new(client: WrappedClient<M>, pool: Pool<M>) -> Self {
        Self { client: Some(client), pool }
    }
}

impl<M> Drop for Connection<M>
where M: DbClient + Send + 'static
{
    fn drop(&mut self) {
        self.pool.release(self.client.take().unwrap());
    }
}
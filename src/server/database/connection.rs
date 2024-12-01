use tokio_postgres::{Client, Error, Row, ToStatement, Transaction};
use tokio_postgres::types::ToSql;
use crate::server::database::pool::{Pool, DbClient, GenericTransaction, WrappedTransaction, GenericRow, WrappedRow};

#[cfg(test)]
use crate::server::database::pool::{MockRow, MockTransaction};

pub(crate) struct Connection<M>
where M: DbClient<Client = M>
{
    pub(crate) client: Option<M::Client>,
    pub(crate) pool: Pool<M>,
}

#[cfg(not(test))]
impl DbClient for Client {
    type Client = Client;

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

    async fn transaction(&mut self) -> Result<WrappedTransaction<Transaction>, Error>
    {
        let txn = self.transaction().await;
        match txn {
            Ok(txn) => Ok(WrappedTransaction(txn)),
            Err(e) => Err(e),
        }
    }
}


/// for test
#[cfg(test)]
pub(crate) struct MockClient;
#[cfg(test)]
impl DbClient for MockClient {
    type Client = MockClient;

    #[allow(unused_variables)]
    async fn query<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<impl GenericRow>, Error>
    where
        T: ?Sized + ToStatement
    {
        println!("query mock client");
        Ok(Vec::<MockRow>::new())
    }

    #[allow(unused_variables)]
    async fn execute<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement
    {
        println!("execute mock client");
        Ok(u64::MIN)
    }

    async fn transaction(&mut self) -> Result<MockTransaction, Error> {
        Ok(MockTransaction::new())
    }
}

#[cfg(not(test))]
impl GenericTransaction<Row> for Transaction<'_> {
    async fn query_one<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<WrappedRow<Row>, Error>
    where
        T: ?Sized + ToStatement
    {
        self.query_one(statement, params).await.map(|row| WrappedRow(row))
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
where M : DbClient<Client = M>
{
    pub fn new(client: M::Client, pool: Pool<M>) -> Self {
        Self { client: Some(client), pool }
    }
}

impl<M> Drop for Connection<M>
where M: DbClient<Client = M>
{
    fn drop(&mut self) {
        self.pool.release(self.client.take().unwrap());
    }
}
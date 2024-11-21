use anyhow::Error;
use std::env;
use tokio_postgres::NoTls;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("src/server/database/migrations");
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let conn_str = env::var("MIGRATION_CONNECTION_STR")
        .unwrap_or("postgresql://postgres:pass@localhost".parse()?);
    let (mut client, conn) = tokio_postgres::connect(conn_str.as_str(), NoTls)
        .await
        .expect("failed to connect to db, aborting");
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("connection error: {}", e);
        }
    });
    let report = embedded::migrations::runner()
        .run_async(&mut client)
        .await?;
    println!("report={:?}", report);
    Ok(())
}

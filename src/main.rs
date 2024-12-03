//! application entry point

use crate::server::model::config::ServerConfig;
use log::info;
use std::env;
use std::net::SocketAddrV4;
use std::path::Path;
use std::str::FromStr;
use derive_more::Display;
use tokio_postgres::Client;

mod server;

const DOTENV_LOADING_FAILED_MSG: &str = "failed to load envs from dotenv files, aborting";
const HOST_PARSING_FAILED_MSG: &str = "failed to parse HOST, aborting";
const DEFAULT_HOST_ADDR: &str = "127.0.0.1:8080";
const DEFAULT_DB_READ_POOL_CONN_STR: &str = "postgresql://postgres:pass@localhost";
const DEFAULT_DB_WRITE_POOL_CONN_STR: &str = "postgresql://postgres:pass@localhost"; // TODO:use different user from read pool

#[actix_web::main()]
async fn main() -> std::io::Result<()> {
    // bootstrap
    // a. env
    let env = env::var("APP_ENV")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(Env::Dev); // default dev env if absent

    match env {
        Env::Prod | Env::Stg => {} // load in CI
        Env::Dev => dotenvy::from_path(Path::new(".env.dev"))
            .expect(DOTENV_LOADING_FAILED_MSG),
    };

    // b. logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // c. run app
    let (db_read_conn_string, db_write_conn_string) = (
        env::var("DB_READ_POOL_CONN_STR").unwrap_or(DEFAULT_DB_READ_POOL_CONN_STR.to_string()),
        env::var("DB_WRITE_POOL_CONN_STR").unwrap_or(DEFAULT_DB_WRITE_POOL_CONN_STR.to_string()),
    );
    let config = ServerConfig::new(
        SocketAddrV4::from_str(
            env::var("HOST")
                .unwrap_or(DEFAULT_HOST_ADDR.to_string())
                .as_str(),
        )
        .expect(HOST_PARSING_FAILED_MSG),
        db_read_conn_string,
        db_write_conn_string,
    );

    info!("App is starting in env={}", env);

    server::run(config).await
}

#[derive(Debug, Display)]
#[non_exhaustive]
enum Env {
    Dev,
    Stg,
    Prod,
}

impl FromStr for Env {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dev" => Ok(Self::Dev),
            "stg" => Ok(Self::Stg),
            "prod" => Ok(Self::Prod),
            s => Err(format!("Invalid Env: {s}")),
        }
    }
}

//! application entry point

use std::cmp::{Ordering, PartialOrd};
use std::env;
use std::fmt::{Display, Formatter, Pointer};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::Path;
use std::str::FromStr;
use log::info;
use crate::server::model::config::ServerConfig;

mod server;

const DOTENV_LOADING_FAILED_MSG: &'static str = "failed to load envs from dotenv files, aborting";
const HOST_PARSING_FAILED_MSG: &'static str = "failed to parse HOST, aborting";
const HOST_DEFAULT_ADDR: &'static str = "127.0.0.1:8080";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // bootstrap
    // a. env
    let env = env::var("APP_ENV")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(Env::Dev); // default dev env if absent

    match env {
        Env::Prod => {}, // load in CI
        Env::Stg => dotenvy::from_path(Path::new("src/server/env/.env.stg")).expect(DOTENV_LOADING_FAILED_MSG),
        Env::Dev => dotenvy::from_path(Path::new("src/server/env/.env.dev")).expect(DOTENV_LOADING_FAILED_MSG),
    };

    // b. logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // c. run app
    let config = ServerConfig::new(
        SocketAddrV4::from_str(env::var("HOST").unwrap_or(HOST_DEFAULT_ADDR.to_string()).as_str()).expect(HOST_PARSING_FAILED_MSG),
    );

    info!("App is starting in env={:?}", env);

    server::run(config).await
}

#[derive(Debug, PartialOrd, PartialEq)]
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

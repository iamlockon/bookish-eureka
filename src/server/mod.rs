//! main file for the server

pub mod model;
mod routes;

use actix_web::{middleware::Logger, App, HttpServer};
use crate::server::model::config::ServerConfig;
use crate::server::routes::orders::get_orders;

/// Run the server
pub async fn run(ServerConfig{ addr }: ServerConfig) -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(get_orders)
    })
        .bind(addr)?
        .run()
        .await
}
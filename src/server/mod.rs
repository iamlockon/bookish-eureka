//! main file for the server

mod controller;
mod database;
pub mod model;
mod state;
pub(crate) mod util;

use crate::server::database::pool::PgPool;
use crate::server::model::config::ServerConfig;
use crate::server::state::AppState;
use actix_web::{middleware::Logger, web, App, HttpServer};
use std::sync::{Arc, OnceLock};
use crate::server::controller::bill::{delete_bill_items, get_bill, post_bill_items};
use crate::server::controller::table::{get_tables, patch_table};

static APP_STATE: OnceLock<AppState> = OnceLock::new();

/// Run the server
pub async fn run(
    ServerConfig {
        addr,
        db_read_pool_conn_str,
        db_write_pool_conn_str,
    }: ServerConfig,
) -> std::io::Result<()> {
    // init app state, only one thread
    APP_STATE
        .set(AppState::new(
            Arc::new(
                PgPool::new(db_read_pool_conn_str)
                    .await
                    .expect("failed to init db pool for read"),
            ),
            Arc::new(
                PgPool::new(db_write_pool_conn_str)
                    .await
                    .expect("failed to init db pool for write"),
            ),
        ))
        .ok();

    let app_state = web::Data::new(APP_STATE.get().expect("failed to get app state"));
    // init http server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(app_state.clone())
            .service(get_tables)
            .service(get_bill)
            .service(patch_table)
            .service(post_bill_items)
            .service(delete_bill_items)
    })
    .bind(addr)?
    .run()
    .await
}

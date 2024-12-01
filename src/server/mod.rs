//! main file for the server

mod controller;
mod database;
pub mod model;
mod state;
pub(crate) mod util;
mod scheduler;

use crate::server::database::pool::{DbClient, Init, Pool};
use crate::server::model::config::ServerConfig;
use crate::server::state::AppState;
use actix_web::{middleware::Logger, web, App, HttpServer};
use std::sync::{OnceLock};
use log::error;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use crate::server::controller::bill::{delete_bill_items, get_bill, post_bill_items};
use crate::server::controller::table::{get_tables, patch_table};
use crate::server::scheduler::job::bill_item_sweeper;

static APP_STATE: OnceLock<AppState> = OnceLock::new();

/// Run the server
pub async fn run(
    ServerConfig {
        addr,
        db_read_pool_conn_str,
        db_write_pool_conn_str,
    }: ServerConfig,
) -> std::io::Result<()> {
    let read_pool= {
        let mut pool = Pool::new().await.unwrap();
        pool.init(db_read_pool_conn_str).await.ok();
        pool
    };
    let write_pool= {
        let mut pool = Pool::new().await.unwrap();
        pool.init(db_write_pool_conn_str).await.ok();
        pool
    };
    
    APP_STATE
        .set(AppState::new(
            read_pool,
            write_pool,
        ))
        .ok();

    let db_jobs_handle=  tokio::spawn(db_jobs());
    
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
    .await?;

    db_jobs_handle.await?
}

async fn db_jobs() -> std::io::Result<()> {
    let cancel_token = CancellationToken::new();
    let bill_item_sweeper_handle = tokio::spawn(bill_item_sweeper(cancel_token.clone()));
    match signal::ctrl_c().await {
        Ok(()) => {
            cancel_token.cancel();
            bill_item_sweeper_handle.await.expect("failed to gracefully shutdown bill item sweeper");
        },
        Err(err) => {
            error!("failed to await termination signal, {}", err);
        },
    }
    Ok(())
}

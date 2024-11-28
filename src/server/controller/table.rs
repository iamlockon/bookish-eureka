use std::time::Duration;
use actix_web::{get, patch, web, Responder};
use actix_web::rt::time;
use anyhow::Context;
use log::{error, info, warn};
use tokio_postgres::types::{ToSql};
use crate::server::controller::error::CustomError;
use crate::server::controller::error::CustomError::{BadRequest, DbError, Timeout};
use crate::server::controller::DB_TIMEOUT_SECONDS;
use crate::server::model::table::{GetTablesResponse, PatchTablesResponse, Table};
use crate::server::state::AppState;

#[patch("/v1/table/{id}")]
/// occupy a table
async fn patch_table(
    id: web::Path<i16>,
    data: web::Data<&AppState>,
) -> Result<impl Responder, CustomError> {
    if let Some(mut conn) = data.get_db_write_pool().acquire().await {
        let client = conn.client.as_mut().unwrap();
        match client.transaction().await.context("failed to start transaction") {
            Ok(txn) => {
                let id = id.into_inner();
                let params: &[&(dyn ToSql + Sync)] = &[&id];
                // check table availability
                let sleep = time::sleep(Duration::from_secs(DB_TIMEOUT_SECONDS));
                tokio::pin!(sleep);
                tokio::select! {
                    result = txn.query_one(r#"SELECT bill_id FROM "table" WHERE id = $1 FOR UPDATE"#, params) => {
                        match result {
                            Ok(table) => {
                                match table.try_get::<&str, Option<i64>>("bill_id") {
                                    Ok(Some(bill_id)) => {
                                        warn!("the table is already taken, bill_id={}", bill_id);
                                        return Err(BadRequest);
                                    },
                                    Ok(None) => {
                                        info!("table {} is available, continue to prepare table...", id);
                                    },
                                    Err(e) => {
                                        warn!("query error, {}", e);
                                        return Err(DbError);
                                    }
                                }
                            },
                            Err(e) => {
                                error!("failed to query, {}", e);
                                return Err(DbError);
                            }
                        }
                    },
                    _ = &mut sleep => {
                        warn!("timeout when trying to select table for update");
                        return Err(Timeout);
                    }
                }

                // insert bill
                match txn.query_one(r#"
                    INSERT INTO bill(table_id, created_at)
                    VALUES ($1, $2)
                    RETURNING id, table_id
                "#, &[&id, &crate::server::util::time::helper::get_utc_now()]).await {
                    Ok(row) => {
                        // bind bill to table
                        let bill_id = row.get("id");
                        match txn.execute(r#"
                            UPDATE "table" ta
                            SET bill_id = $2
                            WHERE ta.id = $1
                        "#, &[&id, &bill_id]).await {
                            Ok(_) => {
                                // save the work
                                txn.commit().await.map_err(|_| DbError)?;
                                Ok(web::Json(PatchTablesResponse {
                                    bill_id,
                                }))
                            },
                            Err(e) => {
                                error!("failed to bind bill to table, {}", e);
                                Err(DbError)
                            }
                        }
                    },
                    Err(e) => {
                        error!("failed to insert bill, {}", e);
                        Err(DbError)
                    }
                }
            },
            Err(e) => {
                error!("db error, {}", e);
                Err(DbError)
            }
        }
    } else {
        Err(CustomError::ServerIsBusy)
    }
}

#[get("/v1/tables")]
/// get tables
async fn get_tables(data: web::Data<&AppState>) -> Result<impl Responder, CustomError> {
    if let Some(conn) = data.get_db_read_pool().acquire().await {
        let client = conn.client.as_ref().unwrap();
        return match client.query(r##"
            SELECT t.id as t_id, b.id as b_id, CASE WHEN b.id IS NULL THEN 'Y' ELSE 'N' END AS availabie
            FROM "table" t
            LEFT JOIN bill b
            on t.bill_id = b.id
            ;
        "##, &[]).await {
            Ok(rows) => {
                let tables = rows.into_iter()
                    .map(|r| {
                        Table {
                            id: r.get::<&str, i16>("t_id") as u8,
                            bill_id: r.try_get::<&str, i64>("b_id").ok(),
                        }
                    })
                    .collect::<Vec<_>>();

                    Ok(web::Json(GetTablesResponse {
                        tables: Some(tables),
                    }))
            }
            Err(e) => {
                error!("get_tables failed, {}", e);
                Err(DbError)
            }
        };
    }
    Err(CustomError::ServerIsBusy)
}

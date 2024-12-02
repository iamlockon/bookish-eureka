use crate::server::database::pool::GenericTransaction;
use crate::server::database::pool::GenericRow;
use crate::server::database::pool::DbClient;
use std::time::Duration;
use actix_web::{get, patch, post, put, web, Responder};
use actix_web::rt::time;
use log::{error, info, warn};
use tokio_postgres::types::{ToSql};
use crate::server::controller::error::CustomError;
use crate::server::controller::error::CustomError::{BadRequest, DbError, Timeout};
use crate::server::controller::DB_TIMEOUT_SECONDS;
use crate::server::model::table::{GetTablesResponse, PatchTablesResponse, PostTablesResponse, Table};
use crate::server::state::AppState;

#[patch("/v1/table/{id}")]
/// occupy a table
async fn patch_table(
    id: web::Path<i16>,
    data: web::Data<&AppState>,
) -> Result<impl Responder, CustomError> {
    if let Some(mut conn) = data.get_db_write_pool().acquire().await {
        let client = conn.client.as_mut().unwrap();
        match client.transaction().await {
            Ok(txn) => {
                let id = id.into_inner();
                let params: &[&(dyn ToSql + Sync)] = &[&id];
                // check table availability
                let sleep = time::sleep(Duration::new(DB_TIMEOUT_SECONDS, 0));
                tokio::pin!(sleep);
                tokio::select! {
                    result = txn.query_one(r#"SELECT bill_id FROM "table" WHERE id = $1 FOR UPDATE"#, params) => {
                        match result {
                            Ok(row) => {
                                match row.try_get::<&str, Option<i64>>("bill_id") {
                                    Ok(Some(bill_id)) => {
                                        warn!("the table is already taken, bill_id={}", bill_id);
                                        return Err(BadRequest);
                                    },
                                    Ok(None) => {
                                        info!("table {} is available, continue to prepare table...", id);
                                    },
                                    Err(e) => {
                                        warn!("query error, {}", e);
                                        return Err(DbError(e.into()));
                                    }
                                }
                            },
                            Err(e) => {
                                error!("failed to query, {}", e);
                                return Err(DbError(e.into()));
                            }
                        }
                    },
                    _ = &mut sleep => {
                        warn!("timeout when trying to select table for update");
                        return Err(Timeout);
                    }
                }

                // insert bill
                let result: Result<i64, CustomError> = match txn.query_one(r#"
                    INSERT INTO bill(table_id, created_at)
                    VALUES ($1, $2)
                    RETURNING id, table_id
                "#, &[&id as &(dyn ToSql + Sync), &crate::server::util::time::helper::get_utc_now() as &(dyn ToSql + Sync)]).await {
                    Ok(row) => {
                        // bind bill to table
                        let bill_id: i64 = row.get("id");
                        match txn.execute(r#"
                            UPDATE "table" ta
                            SET bill_id = $2
                            WHERE ta.id = $1
                        "#, &[&id as &(dyn ToSql + Sync), &bill_id]).await {
                            Ok(_) => {
                                Ok(bill_id)
                            },
                            Err(e) => {
                                error!("failed to bind bill to table, {}", e);
                                Err(DbError(e.into()))
                            }
                        }
                    },
                    Err(e) => {
                        error!("failed to insert bill, {}", e);
                        Err(DbError(e.into()))
                    }
                };
                match result {
                    Ok(bill_id) => {
                        txn.commit().await.map_err(|e| DbError(e.into()))?;
                        Ok(web::Json(PatchTablesResponse {
                            bill_id,
                        }))
                    },
                    Err(e) => Err(e)
                }
            },
            Err(e) => {
                error!("db error, {}", e);
                Err(DbError(e.into()))
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
                Err(DbError(e.into()))
            }
        };
    }
    Err(CustomError::ServerIsBusy)
}


#[post("/v1/table/{id}")]
/// checkout a table
async fn post_table(
    id: web::Path<i16>,
    data: web::Data<&AppState>,
) -> Result<impl Responder, CustomError> {
    if let Some(mut conn) = data.get_db_write_pool().acquire().await {
        let client = conn.client.as_mut().unwrap();
        match client.transaction().await {
            Ok(txn) => {
                let id = id.into_inner();
                let params: &[&(dyn ToSql + Sync)] = &[&id];
                // check table is eligible for checkout
                let sleep = time::sleep(Duration::new(DB_TIMEOUT_SECONDS, 0));
                tokio::pin!(sleep);
                tokio::select! {
                    result = txn.query_one(r#"SELECT bill_id FROM "table" WHERE id = $1 FOR UPDATE"#, params) => {
                        match result {
                            Ok(row) => {
                                match row.try_get::<&str, Option<i64>>("bill_id") {
                                    Ok(Some(bill_id)) => {
                                        info!("the table :[{}] is eligible for checkout, bill_id={}. Will continue to checkout.", id, bill_id);
                                        // update the bill
                                        match txn.query_one(r#"
                                            UPDATE bill
                                            SET checkout_at = CURRENT_TIMESTAMP
                                            WHERE id = $1
                                            RETURNING id
                                        "#, &[&bill_id]).await {
                                            Ok(row) => {
                                                let bill_id = row.get::<&str, i16>("id");
                                                info!("checkout table {} with bill id {} successfully", id, bill_id);
                                            },
                                            Err(e) => {
                                                error!("failed to checkout table {}, {}", id, e);
                                                return Err(DbError(e.into()));
                                            }
                                        }
                                    },
                                    Ok(None) => {
                                        warn!("table {} does not have a bill", id);
                                        return Err(BadRequest);
                                    },
                                    Err(e) => {
                                        warn!("query error, {}", e);
                                        return Err(DbError(e.into()));
                                    }
                                }
                            },
                            Err(e) => {
                                error!("failed to query, {}", e);
                                return Err(DbError(e.into()));
                            }
                        }
                    },
                    _ = &mut sleep => {
                        warn!("timeout when trying to select table for update");
                        return Err(Timeout);
                    }
                }

                // detach bill
                let result: Result<i16, CustomError> = match txn.query_one(r#"
                    UPDATE "table"
                    SET bill_id = NULL
                    WHERE id = $1
                    RETURNING id
                "#, &[&id]).await {
                    Ok(row) => {
                        let id = row.get::<&str, i16>("id");
                        info!("checkout table {} successfully", id);
                        Ok(id)
                    },
                    Err(e) => {
                        error!("failed to checkout table {}, {}", id, e);
                        Err(DbError(e.into()))
                    }
                };
                
                match result {
                    Ok(id) => {
                        txn.commit().await.map_err(|e| DbError(e.into()))?;
                        Ok(web::Json(PostTablesResponse {
                            id: id as u8
                        }))
                    },
                    Err(e) => Err(e)
                }
            },
            Err(e) => {
                error!("db error, {}", e);
                Err(DbError(e.into()))
            }
        }
    } else {
        Err(CustomError::ServerIsBusy)
    }
}

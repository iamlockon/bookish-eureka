use crate::server::database::pool::GenericRow;
use std::ops::{RangeInclusive};
use std::time::Duration;
use crate::server::model::bill::{Bill, GetBillResponse, PostBillItemsRequest};
use crate::server::state::AppState;
use actix_web::{delete, get, post, web, HttpRequest, HttpResponse, Responder};
use actix_web::rt::time;
use anyhow::Context;
use log::{error, info, warn};
use rand::Rng;
use tokio_postgres::{Client, Row};
use tokio_postgres::error::SqlState;
use tokio_postgres::types::ToSql;
use crate::server::controller::error::CustomError;
use crate::server::DB_TIMEOUT_SECONDS;
use crate::server::model::CommonRequestParams;
use crate::server::model::item::Item;
use crate::server::database::pool::DbClient;

#[post("/v1/bill/{id}/items")]
/// Add bill associated items
async fn post_bill_items(id: web::Path<i64>, body: web::Json<PostBillItemsRequest>, data: web::Data<&AppState>) -> Result<impl Responder, CustomError> {
    const COLUMN_LEN: usize = 5;
    const TIME_TO_DELIVER_RANGE: RangeInclusive<i32> = 5..=15;
    if let Some(mut conn) = data.get_db_write_pool().acquire(DB_TIMEOUT_SECONDS).await {
        let mut stmt = "INSERT INTO bill_item(bill_id, menu_item_id, state, time_to_deliver, created_at) VALUES".to_string();
        let mut idx = 1;
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(body.items.len() * COLUMN_LEN);
        let id = id.into_inner();
        let rand_v = (0..body.items.len()).fold(Vec::with_capacity(body.items.len()), |mut acc, _| {
            acc.push(rand::thread_rng().gen_range(TIME_TO_DELIVER_RANGE));
            acc
        }).into_iter().collect::<Vec<_>>();
        let created_at = crate::server::util::time::helper::get_utc_now();
        for (i, menu_item_id) in body.items.iter().enumerate() {
            let maybe_comma = if i != body.items.len() - 1 { "," } else { "" };
            stmt.extend(format!(" (${}, ${}, ${}, ${}, ${}){}", idx, idx+1, idx+2, idx+3, idx+4, maybe_comma).chars());
            let cur_params = [&id as &(dyn ToSql + Sync), menu_item_id as &(dyn ToSql + Sync), &"created", &rand_v[i], &created_at];
            params.extend(cur_params.into_iter());
            idx += COLUMN_LEN;
        }

        stmt.extend(" RETURNING id".chars());

        let client = match &mut conn.client {
            Some(client) => client,
            None => {
                error!("client is None");
                return Err(CustomError::Unknown);
            },
        };
        match client.query(&stmt, &params.as_slice()).await {
            Ok(rows) => rows,
            Err(e) => {
                match e.code().unwrap() {
                    &SqlState::FOREIGN_KEY_VIOLATION => {
                        warn!("the requested bill or menu item does not exist");
                        return Err(CustomError::BadRequest);
                    },
                    code => {
                        error!("unhandled db error, code={:?}", code);
                    },
                };
                return Err(CustomError::DbError(e.into()));
            },
        };

        return Ok(HttpResponse::Ok());

    }
    Err(CustomError::ServerIsBusy)
}

#[delete("/v1/bill/{id}/item/{item_id}")]
/// Remove one specific bill item
async fn delete_bill_items(path: web::Path<(i64, i64)>, data: web::Data<&AppState>) -> Result<impl Responder, CustomError> {
    let (id, item_id) = path.into_inner();
    let params: &[&(dyn ToSql + Sync)] = &[&item_id, &id];
    if let Some(conn) = data.get_db_write_pool().acquire(DB_TIMEOUT_SECONDS).await {
        let sleep = time::sleep(Duration::new(DB_TIMEOUT_SECONDS, 0));
        tokio::pin!(sleep);
        let client = conn.client.as_ref().unwrap();
        return tokio::select! {
            result = client.execute(r#"
                UPDATE bill_item SET state = 'deleted'
                WHERE id = $1 AND bill_id = $2
            "#, params) => {
                match result {
                    Ok(0) => Err(CustomError::ResourceNotFound),
                    Ok(_) => Ok(HttpResponse::Ok()),
                    Err(e) => {
                        warn!("delete_bill_items failed, {}", e);
                        Err(CustomError::DbError(e.into()))
                    }
                }
            },
            _ = &mut sleep => {
                warn!("timeout deleting a bill item");
                return Err(CustomError::Timeout);
            }
        }
    }
    Err(CustomError::ServerIsBusy)
}

#[get("/v1/bill/{id}")]
/// get bill items
async fn get_bill(id: web::Path<i64>, req: HttpRequest, data: web::Data<&AppState>) -> Result<impl Responder, CustomError> {
    if let Some(conn) = data.get_db_read_pool().acquire(DB_TIMEOUT_SECONDS).await {
        let maybe_queries = web::Query::<CommonRequestParams>::from_query(req.query_string()).context("failed to parse query string");
        if maybe_queries.is_err() {
            return Err(CustomError::BadRequest);
        }
        let CommonRequestParams {
            page: maybe_page, 
            page_size: maybe_page_size
        } = maybe_queries.unwrap().into_inner();
        let (page, page_size) = (maybe_page.unwrap_or(0), maybe_page_size.unwrap_or(20));
        let id = id.into_inner();
        let client = conn.client.as_ref().unwrap();
        return match client.query(r##"
            SELECT b.id, mi.name, b.time_to_deliver, b.state
            FROM bill_item b
            JOIN menu_item mi
            ON b.menu_item_id = mi.id
            WHERE bill_id = $1 AND b.state IS DISTINCT FROM 'deleted'
            OFFSET $2
            LIMIT $3
            ;
        "##, &[&id, &(page as i64) as &(dyn ToSql + Sync), &(page_size as i64) as &(dyn ToSql + Sync)]).await {
            Ok(rows) => {
                let items = rows.into_iter().map_while(|r|
                    match (r.try_get("id"), r.try_get("name"), r.try_get("time_to_deliver"), r.try_get("state")) {
                        (Ok(id), Ok(name), Ok(time_to_deliver), Ok(state)) => {
                            Some(Item {
                                id, name, time_to_deliver, state
                            })
                        },
                        _ => {
                            None
                        }
                    }
                ).collect::<Vec<_>>();
                
                Ok(web::Json(GetBillResponse {
                    bill: match items.is_empty() {
                        true => None,
                        false => Some(Bill {
                            id,
                            items,
                        })
                    },
                }))
            }
            Err(e) => {
                error!("get_bills failed, {}", e);
                Err(CustomError::DbError(e.into()))
            }
        };
    }
    Err(CustomError::ServerIsBusy)
}

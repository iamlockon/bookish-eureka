use std::time::Duration;
use crate::server::model::bill::{Bill, GetBillResponse, PostBillItemsRequest};
use crate::server::state::AppState;
use actix_web::{delete, get, post, web, HttpRequest, HttpResponse, Responder};
use actix_web::rt::time;
use anyhow::Context;
use log::{error, warn};
use tokio_postgres::types::ToSql;
use crate::server::controller::error::CustomError;
use crate::server::controller::DB_TIMEOUT_SECONDS;
use crate::server::model::CommonRequestParams;
use crate::server::model::item::Item;

#[post("/v1/bill/{id}/items")]
/// Add bill associated items
async fn post_bill_items(id: web::Path<i64>, body: web::Json<PostBillItemsRequest>, data: web::Data<&AppState>) -> Result<impl Responder, CustomError> {
    const COLUMN_LEN: usize = 3;
    if let Some(conn) = data.get_db_write_pool().acquire().await {
        let mut stmt = "INSERT INTO bill_item(bill_id, menu_item_id, state) VALUES".to_string();
        let mut idx = 1;
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(body.items.len() * COLUMN_LEN);
        let id = id.into_inner();
        for (i, menu_item_id) in body.items.iter().enumerate() {
            let maybe_comma = if i != body.items.len() - 1 { "," } else { "" };
            stmt.extend(format!(" (${}, ${}, ${}){}", idx, idx+1, idx+2, maybe_comma).chars());
            params.extend(&[&id as &(dyn ToSql + Sync), menu_item_id as &(dyn ToSql + Sync), &"created"]);
            idx += COLUMN_LEN;
        }
        let client = conn.client.as_ref().unwrap();
        return match client
            .execute(&stmt, &params.as_slice())
            .await
        {
            Ok(_) => Ok(HttpResponse::Ok()),
            Err(e) => {
                warn!("post_bill_items failed, {}", e);
                Err(CustomError::DbError)
            }
        };
    }
    Err(CustomError::ServerIsBusy)
}

#[delete("/v1/bill/{id}/item/{item_id}")]
/// Remove one specific bill item
async fn delete_bill_items(path: web::Path<(i64, i32)>, data: web::Data<&AppState>) -> Result<impl Responder, CustomError> {
    let (id, item_id) = path.into_inner();
    let params: &[&(dyn ToSql + Sync)] = &[&id, &item_id];
    if let Some(conn) = data.get_db_write_pool().acquire().await {
        let sleep = time::sleep(Duration::from_secs(DB_TIMEOUT_SECONDS));
        tokio::pin!(sleep);
        let client = conn.client.as_ref().unwrap();
        return tokio::select! {
            result = client.execute(r#"
                UPDATE bill_item SET state = 'deleted'
                WHERE id = (
                    SELECT id
                    FROM bill_item
                    WHERE state IS DISTINCT FROM 'deleted'
                    AND bill_id = $1
                    AND menu_item_id = $2
                    LIMIT 1
                    FOR UPDATE
                )
            "#, params) => {
                match result {
                    Ok(0) => Err(CustomError::ResourceNotFound),
                    Ok(_) => Ok(HttpResponse::Ok()),
                    Err(e) => {
                        warn!("delete_bill_items failed, {}", e);
                        Err(CustomError::DbError)
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
    if let Some(conn) = data.get_db_read_pool().acquire().await {
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
            SELECT b.id, mi.name
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
                    match (r.try_get("id"), r.try_get("name")) {
                        (Ok(id), Ok(name)) => {
                            Some(Item {
                                id, name
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
                Err(CustomError::DbError)
            }
        };
    }
    Err(CustomError::ServerIsBusy)
}

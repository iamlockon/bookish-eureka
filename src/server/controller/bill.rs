use crate::server::model::bill::{Bill, DeleteBillItemsRequest, DeleteBillItemsResponse, GetBillResponse, ItemModification, PostBillItemsRequest, PostBillItemsResponse};
use crate::server::state::AppState;
use actix_web::{delete, get, post, web, HttpRequest, HttpResponse, Responder};
use anyhow::Context;
use chrono::{Utc};
use log::{error, warn};
use tokio_postgres::types::ToSql;
use crate::server::controller::error::CustomError;
use crate::server::model::CommonRequestParams;
use crate::server::model::item::Item;

#[post("/v1/bill/{id}/items")]
/// Add bill associated items
async fn post_bill_items(id: web::Path<i64>, body: web::Json<PostBillItemsRequest>, data: web::Data<&AppState>) -> Result<impl Responder, CustomError> {
    const COLUMN_LEN: usize = 3;
    if let Some(conn) = data.get_db_read_pool().acquire().await {
        let mut stmt = "INSERT INTO bill_item(bill_id, menu_item_id, count) VALUES".to_string();
        let mut idx = 1;
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(body.items.len() * COLUMN_LEN);
        let id = id.into_inner();
        for ItemModification { id: item_id, count } in &body.items {
            stmt.extend(format!(" (${}, ${}, ${})", idx, idx+1, idx+2).chars());
            params.extend(&[&id as &(dyn ToSql + Sync), item_id as &(dyn ToSql + Sync), count as &(dyn ToSql + Sync)]);
            idx += COLUMN_LEN;
        }
        return match conn
            .client
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

#[delete("/v1/bill/{id}/items")]
/// Remove some bill associated items
async fn delete_bill_items(id: web::Path<i64>, body: web::Json<DeleteBillItemsRequest>, data: web::Data<&AppState>) -> Result<impl Responder, CustomError> {

    if let Some(conn) = data.get_db_read_pool().acquire().await {
        let mut ids = Vec::with_capacity(body.items.len());
        for &item_id in &body.items {
            ids.push(item_id.to_string());
        }
        let stmt = format!("UPDATE bill_item SET deleted = true WHERE id IN ({})", ids.join(","));
        return match conn
            .client
            .execute(&stmt,  &[])
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

#[get("/v1/bill/{id}/items")]
/// get bill items
async fn get_bill_items(id: web::Path<i64>, req: HttpRequest, data: web::Data<&AppState>) -> Result<impl Responder, CustomError> {
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
        return match conn.client.query(r##"
            SELECT b.id, mi.name
            FROM bill_item b
            JOIN menu_item mi
            ON b.menu_item_id = mi.id
            WHERE bill_id = $1
            OFFSET $2
            LIMIT $3
            ;
        "##, &[&(page as i32) as &(dyn ToSql + Sync), &(page_size as i32) as &(dyn ToSql + Sync)]).await {
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
                    result_code: None,
                    bill: match items.is_empty() {
                        true => None,
                        false => Some(Bill {
                            id: id.into_inner(),
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

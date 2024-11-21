use crate::server::model::bill::{Bill, GetBillsResponse, PostBillsRequest, PostBillsResponse};
use crate::server::state::AppState;
use crate::server::util::time;
use actix_web::{get, post, web, HttpRequest, Responder};
use chrono::{DateTime, Utc};
use futures_util::{pin_mut, Stream};
use log::{error, info, warn};
use tokio_postgres::types::{ToSql, Type};

#[get("/v1/bills")]
async fn get_bills(data: web::Data<&AppState>) -> impl Responder {
    if let Some(conn) = data.get_db_read_pool().acquire().await {
        return match conn.client.query("SELECT * FROM bill", &[]).await {
            Ok(rows) => {
                info!("bills={:?}", rows);
                (
                    web::Json(GetBillsResponse {
                        result_code: None,
                        bills: rows
                            .into_iter()
                            .map(|r| {
                                let created_at: DateTime<Utc> = r.get("created_at");
                                let updated_at: Option<DateTime<Utc>> = r.get("updated_at");
                                let checkout_at: Option<DateTime<Utc>> = r.get("checkout_at");
                                
                                Bill {
                                    id: r.get("id"),
                                    table_id: r.get("table_id"),
                                    created_at: created_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
                                    updated_at: updated_at.map(|ts| ts.format("%Y-%m-%dT%H:%M:%S").to_string()),
                                    checkout_at: checkout_at.map(|ts| ts.format("%Y-%m-%dT%H:%M:%S").to_string()),
                                    customer_count: r.get("customer_count"),
                                }
                            })
                            .collect::<Vec<Bill>>(),
                    }),
                    http::StatusCode::OK,
                )
            }
            Err(e) => {
                error!("get_bills failed, {}", e);
                (
                    web::Json(GetBillsResponse {
                        result_code: None,
                        bills: vec![],
                    }),
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        };
    }
    (
        web::Json(GetBillsResponse {
            result_code: Some("S1234".to_string()),
            bills: vec![],
        }),
        http::StatusCode::TOO_MANY_REQUESTS,
    )
}

#[get("/v1/bill/{id}")]
async fn get_bill(bill_id: web::Path<i64>, data: web::Data<&AppState>) -> impl Responder {
    if let Some(conn) = data.get_db_read_pool().acquire().await {
        let params = &[bill_id.as_ref() as &(dyn ToSql + Sync)];
        return match conn
            .client
            .query_one("SELECT * FROM bill WHERE id = $1", params)
            .await
        {
            Ok(row) => {
                let created_at: DateTime<Utc> = row.get("created_at");
                let updated_at: Option<DateTime<Utc>> = row.get("updated_at");
                let checkout_at: Option<DateTime<Utc>> = row.get("checkout_at");

                (
                    web::Json(GetBillsResponse {
                        result_code: None,
                        bills: vec![Bill {
                            id: row.get("id"),
                            table_id: row.get("table_id"),
                            created_at: created_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
                            updated_at: updated_at.map(|ts| ts.format("%Y-%m-%dT%H:%M:%S").to_string()),
                            checkout_at: checkout_at.map(|ts| ts.format("%Y-%m-%dT%H:%M:%S").to_string()),
                            customer_count: row.get("customer_count"),
                        }],
                    }),
                    http::StatusCode::OK,
                )
            }
            Err(e) => {
                warn!("get_bill failed, {}", e);
                (
                    web::Json(GetBillsResponse {
                        result_code: None,
                        bills: vec![],
                    }),
                    http::StatusCode::NOT_FOUND,
                )
            }
        };
    }
    (
        web::Json(GetBillsResponse {
            result_code: Some("S1234".to_string()),
            bills: vec![],
        }),
        http::StatusCode::TOO_MANY_REQUESTS,
    )
}

#[post("/v1/bills")]
async fn post_bills(
    req: web::Json<PostBillsRequest>,
    data: web::Data<&AppState>,
) -> impl Responder {
    if let Some(conn) = data.get_db_write_pool().acquire().await {
        let params: &[&(dyn ToSql + Sync); 3] = &[
            &req.table_id,
            &req.customer_count,
            &time::helper::get_utc_now(),
        ];
        return match conn
            .client
            .execute(
                r#"
                INSERT INTO bill(table_id, customer_count, created_at)
                VALUES ($1, $2, $3);
            "#,
                params,
            )
            .await
        {
            Ok(_) => (
                web::Json(PostBillsResponse { result_code: None }),
                http::StatusCode::OK,
            ),
            Err(e) => {
                error!("post_bills failed, {}", e);
                (
                    web::Json(PostBillsResponse {
                        result_code: Some("E0000".to_string()),
                    }),
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        };
    }
    (
        web::Json(PostBillsResponse {
            result_code: Some("S1234".to_string()),
        }),
        http::StatusCode::TOO_MANY_REQUESTS,
    )
}

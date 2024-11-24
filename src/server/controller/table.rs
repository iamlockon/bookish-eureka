use actix_web::{get, patch, web, Responder};
use chrono::{Utc};
use log::{error, warn};
use tokio_postgres::types::ToSql;
use crate::server::model::table::{GetTablesResponse, PatchTablesRequest, PatchTablesResponse, Table};
use crate::server::state::AppState;

#[patch("/v1/table/{id}")]
/// occupy a table
async fn patch_table(
    req: web::Json<PatchTablesRequest>,
    id: web::Path<i32>,
    data: web::Data<&AppState>,
) -> impl Responder {
    if let Some(conn) = data.get_db_write_pool().acquire().await {
        let params: &[&(dyn ToSql + Sync); 3] = &[
            &id.into_inner(),
            &req.customer_count,
            &crate::server::util::time::helper::get_utc_now(),
        ];

        return match conn
            .client
            .execute(
                r#"
                WITH input(table_id, customer_count, created_at) as (
                    VALUES ($1, $2, $3)
                ),
                b as (
                    INSERT INTO bill(table_id, customer_count, created_at)
                    SELECT i.table_id, i.customer_count, i.created_at
                    FROM input i
                    RETURNING id, table_id
                )
                UPDATE "table" ta
                SET bill_id = b.id
                FROM b
                WHERE ta.id = b.table_id;
            "#,
                params,
            )
            .await
        {
            Ok(_) => (
                web::Json(PatchTablesResponse { result_code: None }),
                http::StatusCode::OK,
            ),
            Err(e) => {
                error!("patch_table failed, {}", e);
                (
                    web::Json(PatchTablesResponse {
                        result_code: Some("E0000".to_string()),
                    }),
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        };
    }
    (
        web::Json(PatchTablesResponse {
            result_code: Some("S1234".to_string()),
        }),
        http::StatusCode::TOO_MANY_REQUESTS,
    )
}

#[get("/v1/tables")]
/// get tables
async fn get_tables(data: web::Data<&AppState>) -> impl Responder {
    if let Some(conn) = data.get_db_read_pool().acquire().await {
        return match conn.client.query(r##"
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
                            bill_id: r.try_get::<&str, i64>("b_id").ok().map(|id| id as u64),
                        }
                    })
                    .collect::<Vec<_>>();
                (
                    web::Json(GetTablesResponse {
                        result_code: None,
                        tables: Some(tables),
                    }),
                    http::StatusCode::OK,
                )
            }
            Err(e) => {
                error!("get_tables failed, {}", e);
                (
                    web::Json(GetTablesResponse {
                        result_code: None,
                        tables: None,
                    }),
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        };
    }
    (
        web::Json(GetTablesResponse {
            result_code: Some("S1234".to_string()),
            tables: None,
        }),
        http::StatusCode::TOO_MANY_REQUESTS,
    )
}

// #[get("/v1/table/{id}")]
// async fn get_table(id: web::Path<i32>, data: web::Data<&AppState>) -> impl Responder {
//     if let Some(conn) = data.get_db_read_pool().acquire().await {
//         let params = &[&id.into_inner() as &(dyn ToSql + Sync)];
//         return match conn
//             .client
//             .query_one("SELECT id, table_id, created_at, updated_at, checkout_at, customer_count FROM bill WHERE id = $1", params)
//             .await
//         {
//             Ok(row) => {
//                 let created_at: DateTime<Utc> = row.get("created_at");
//                 let updated_at: Option<DateTime<Utc>> = row.get("updated_at");
//                 let checkout_at: Option<DateTime<Utc>> = row.get("checkout_at");
//
//                 (
//                     web::Json(GetBillsResponse {
//                         result_code: None,
//                         bills: vec![Bill {
//                             id: row.get("id"),
//                             table_id: row.get("table_id"),
//                             created_at: created_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
//                             updated_at: updated_at.map(|ts| ts.format("%Y-%m-%dT%H:%M:%S").to_string()),
//                             checkout_at: checkout_at.map(|ts| ts.format("%Y-%m-%dT%H:%M:%S").to_string()),
//                             customer_count: row.get("customer_count"),
//                         }],
//                     }),
//                     http::StatusCode::OK,
//                 )
//             }
//             Err(e) => {
//                 warn!("get_bill failed, {}", e);
//                 (
//                     web::Json(GetBillsResponse {
//                         result_code: None,
//                         bills: vec![],
//                     }),
//                     http::StatusCode::NOT_FOUND,
//                 )
//             }
//         };
//     }
//     (
//         web::Json(GetBillsResponse {
//             result_code: Some("S1234".to_string()),
//             bills: vec![],
//         }),
//         http::StatusCode::TOO_MANY_REQUESTS,
//     )
// }

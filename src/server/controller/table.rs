use actix_web::{get, patch, web, Responder, error, Error, ResponseError, HttpResponse};
use chrono::{Utc};
use log::{error, warn};
use tokio_postgres::types::{ToSql, Type};
use crate::server::controller::error::CustomError;
use crate::server::model::table::{GetTablesResponse, PatchTablesRequest, PatchTablesResponse, Table};
use crate::server::state::AppState;

#[patch("/v1/table/{id}")]
/// occupy a table
async fn patch_table(
    req: web::Json<PatchTablesRequest>,
    id: web::Path<i16>,
    data: web::Data<&AppState>,
) -> Result<impl Responder, CustomError> {
    if let Some(conn) = data.get_db_write_pool().acquire().await {
        let params: &[&(dyn ToSql + Sync); 3] = &[
            &id.into_inner(),
            &req.customer_count,
            &crate::server::util::time::helper::get_utc_now(),
        ];

        return match conn
            .client
            .execute( // TODO: fix with transaction to check table is not taken yet.
                r#"
                WITH input(table_id, customer_count, created_at) as (
                    VALUES ($1::smallint, $2::smallint, $3::timestamptz)
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
                WHERE bill_id IS NULL AND ta.id = b.table_id;
            "#,
                params,
            )
            .await
        {
            Ok(affected_rows) => {
                if affected_rows == 0_u64 { // table is already occupied.
                    return Err(CustomError::BadRequest);
                }
                Ok(HttpResponse::Ok())
            },
            Err(e) => {
                error!("patch_table failed, {}", e);
                Err(CustomError::DbError)
            }
        };
    }
    Err(CustomError::ServerIsBusy)
}

#[get("/v1/tables")]
/// get tables
async fn get_tables(data: web::Data<&AppState>) -> Result<impl Responder, CustomError> {
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

                    Ok(web::Json(GetTablesResponse {
                        result_code: None,
                        tables: Some(tables),
                    }))
            }
            Err(e) => {
                error!("get_tables failed, {}", e);
                Err(CustomError::DbError)
            }
        };
    }
    Err(CustomError::ServerIsBusy)
}

use actix_web::{get, HttpResponse};
use crate::server::model::order::Order;

#[get("/orders")]
async fn get_orders() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .json(Order::default())
}
use actix_web::{error, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub(crate) enum CustomError {
    #[display("server is busy")]
    ServerIsBusy,
    #[display("invalid request")]
    BadRequest,
    #[display("database error")]
    DbError,
    #[display("timeout occurred")]
    Timeout,
}

impl error::ResponseError for CustomError {
    fn status_code(&self) -> StatusCode {
        match *self {
            CustomError::ServerIsBusy | CustomError::DbError => StatusCode::INTERNAL_SERVER_ERROR,
            CustomError::BadRequest => StatusCode::BAD_REQUEST,
            CustomError::Timeout => StatusCode::GATEWAY_TIMEOUT,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }
}

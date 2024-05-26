use actix_web::{HttpResponse, ResponseError};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use derive_more::{Display, Error};
use sqlx::Error;

#[derive(Debug, Display, Error)]
pub enum ApiError {
    #[display(fmt = "Bad request: {}", _0)]
    BadRequest(String),
    #[display(fmt = "Internal server error")]
    InternalServerError,
    #[display(fmt = "The data is not found")]
    NotFound,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    errors: Vec<String>,
}

impl ErrorResponse {
    pub fn new(errors: Vec<String>) -> Self {
        ErrorResponse { errors }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::NotFound => StatusCode::NOT_FOUND,
        }
    }
    fn error_response(&self) -> HttpResponse {
        let error_response = ErrorResponse::new(vec![self.to_string()]);
        let body = serde_json::to_string(&error_response)
            .unwrap_or_else(|_| "{}".to_string());

        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(body)
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: Error) -> ApiError {
        match error {
            Error::RowNotFound => ApiError::NotFound,
            _ => ApiError::InternalServerError,
        }
    }
}
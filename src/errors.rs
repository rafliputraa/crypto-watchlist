use std::fmt;
use std::fmt::Display;
use actix_web::{HttpResponse, ResponseError};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use derive_more::Display;
use sqlx::Error;

#[derive(Debug, Display)]
pub enum ApiError {
    #[display(fmt = "Bad request: {}", _0)]
    BadRequest(String),
    #[display(fmt = "Internal server error")]
    InternalServerError,
    #[display(fmt = "The data is not found")]
    NotFound,
    #[display(fmt = "Invalid Token")]
    InvalidToken,
    #[display(fmt = "Token has expired")]
    ExpiredSignature,
    #[display(fmt = "Missing Token")]
    MissingAuthorizationHeader,
    #[display(fmt = "Malformed Token")]
    MalformedAuthorizationToken,
    #[display(fmt = "Internal server error, Redis: {}", _0)]
    RedisError(String),
    #[display(fmt = "Internal server error, Serde: {}", _0)]
    SerdeError(String),
    #[display(fmt = "Data not found in Redis")]
    RedisNil,
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
            ApiError::InvalidToken => StatusCode::UNAUTHORIZED,
            ApiError::ExpiredSignature => StatusCode::UNAUTHORIZED,
            ApiError::MissingAuthorizationHeader => StatusCode::UNAUTHORIZED,
            ApiError::MalformedAuthorizationToken => StatusCode::UNAUTHORIZED,
            ApiError::RedisError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::SerdeError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::RedisNil => StatusCode::NOT_FOUND,
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

impl From<Error> for ApiError {
    fn from(error: Error) -> ApiError {
        match error {
            Error::RowNotFound => ApiError::NotFound,
            _ => ApiError::InternalServerError,
        }
    }
}

impl From<redis_async::error::Error> for ApiError {
    fn from(err: redis_async::error::Error) -> ApiError {
        ApiError::RedisError(err.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> ApiError {
        ApiError::SerdeError(err.to_string())
    }
}
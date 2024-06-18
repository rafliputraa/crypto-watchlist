use std::future::{ready, Ready};
use std::rc::Rc;
use actix_web::{dev::{Service, ServiceRequest, ServiceResponse, Transform}, Error, HttpMessage, HttpResponse};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, DecodingKey, Validation, TokenData, Algorithm, errors::ErrorKind};
use serde::{Deserialize, Serialize};
use log::{debug, error};
use actix_web::dev::forward_ready;
use crate::config::CONFIG;
use crate::errors::ApiError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i32,
    pub username: String,
    exp: usize,
}

pub struct JWTMiddleware {
    secret: String,
}

impl JWTMiddleware {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JWTMiddleware
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = JwtMiddlewareMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtMiddlewareMiddleware {
            service: Rc::new(service),
            secret: self.secret.clone(),
        }))
    }
}

pub struct JwtMiddlewareMiddleware<S> {
    service: Rc<S>,
    secret: String,
}

impl<S, B> Service<ServiceRequest> for JwtMiddlewareMiddleware<S>
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if req.path() == "/health" {
            return Box::pin(self.service.call(req));
        }

        let secret = self.secret.clone();
        let headers = req.headers().clone();

        let error_response = if headers.get("Authorization").is_none() {
            ApiError::MissingAuthorizationHeader
        } else {
            ApiError::MalformedAuthorizationToken
        };

        if let Some(auth_header) = headers.get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                let token = &auth_str[7..];
                let decoding_key = DecodingKey::from_secret(secret.as_ref());
                let validation = Validation::new(Algorithm::HS256);

                return match decode::<Claims>(token, &decoding_key, &validation) {
                    Ok(TokenData { claims, .. }) => {
                        req.extensions_mut().insert(claims);
                        let fut = self.service.call(req);
                        Box::pin(async move {
                            let res = fut.await?;
                            Ok(res)
                        })
                    }
                    Err(err) => {
                        let error_response = match *err.kind() {
                            ErrorKind::InvalidToken | ErrorKind::InvalidSignature => ApiError::InvalidToken,
                            ErrorKind::ExpiredSignature => ApiError::ExpiredSignature,
                            _ => ApiError::InternalServerError,
                        };
                        Box::pin(async move {
                            Err(error_response.into())
                        })
                    }
                }
            }
        }

        Box::pin(async move {
            Err(error_response.into())
        })
    }
}

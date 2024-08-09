use std::fmt::Debug;
use std::sync::Arc;
use redis_async::client::PairedConnection;
use crate::config::CONFIG;
use futures_util::TryStreamExt;
use kafka::producer::AsBytes;
use redis_async::resp::{FromResp, RespValue};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use crate::errors::ApiError;

#[derive(Debug)]
pub struct Redis {
    redis_client: PairedConnection,
}

#[instrument]
pub async fn create_redis_client() -> Result<Arc<Redis>, ApiError> {
    // Init connection
    let paired_conn = redis_async::client::paired_connect(&CONFIG.redis_host, CONFIG.redis_port).await.unwrap();

    // Authenticate with the Redis server
    let auth_command = vec![
        RespValue::BulkString(b"AUTH".to_vec()),
        RespValue::BulkString(CONFIG.redis_password.as_bytes().to_vec()),
    ];

    let result = paired_conn.send::<RespValue>(RespValue::Array(auth_command)).await.map_err(ApiError::from);

    match result {
        Ok(RespValue::SimpleString(ref s)) if s == "OK" => {

            let redis_client_arc = Arc::new(Redis{ redis_client: paired_conn });
            Ok(redis_client_arc)
        },
        _ => Err(ApiError::RedisError("Authentication failed".into())),
    }
}

impl Redis {

    #[instrument]
    pub async fn del(&self, key: String) -> Result<(), ApiError> {
        let command = vec![
            RespValue::BulkString(b"DEL".to_vec()),
            RespValue::BulkString(key.as_bytes().to_vec()),
        ];

        self.redis_client.send_and_forget(RespValue::Array(command));
        Ok(())
    }

    #[instrument]
    pub async fn set<T: Serialize + Debug>(&self, key: String, value: T) -> Result<(), ApiError> {
        let serialized_value = serde_json::to_string(&value)?;

        let command = vec![
            RespValue::BulkString(b"SET".to_vec()),
            RespValue::BulkString(key.as_bytes().to_vec()),
            RespValue::BulkString(serialized_value.as_bytes().to_vec()),
        ];

        self.redis_client.send_and_forget(RespValue::Array(command));
        Ok(())
    }

    #[instrument]
    pub async fn get<T>(&self, key: String) -> Result<T, ApiError>
    where T: for<'de> Deserialize<'de> + FromResp + Unpin + Debug {

        let command = vec![
            RespValue::BulkString(b"GET".to_vec()),
            RespValue::BulkString(key.as_bytes().to_vec()),
        ];

        let result = self.redis_client.send::<RespValue>(RespValue::Array(command)).await.map_err(ApiError::from);

        match result {
            Ok(RespValue::BulkString(raw_data)) => {
                let value: T = serde_json::from_slice(&raw_data)?;
                Ok(value)
            }
            Ok(RespValue::Nil) => Err(ApiError::RedisNil),
            _ => Err(ApiError::RedisError("Failed to get value".into())),
        }

    }
}
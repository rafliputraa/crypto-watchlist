use std::fmt;
use std::sync::Arc;
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use actix_web::web::{Data, Json, Path};
use redis_async::error::Error;
use redis_async::resp::{FromResp, RespValue};
use sqlx::{Arguments, Executor, Row};
use sqlx::postgres::PgArguments;
use tracing_actix_web::root_span_macro::private::tracing::instrument;
use crate::database::Database;
use crate::errors::ApiError;
use crate::errors::ApiError::{BadRequest, InternalServerError};
use crate::helpers::{respond_json, respond_ok};
use crate::middleware_custom::Claims;
use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WatchlistResponse {
    id: i32,
    name: String,
    symbol: String,
}

impl fmt::Display for WatchlistResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "id: {}, name: {}, symbol: {}", self.id, self.name, self.symbol)
    }
}

impl FromResp for WatchlistResponse {
    fn from_resp(resp: RespValue) -> Result<Self, Error> {
        match resp {
            RespValue::BulkString(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| Error::Internal(e.to_string()))
            },
            _ => Err(Error::Internal("Unexpected response type".to_string())),
        }
    }

    fn from_resp_int(resp: RespValue) -> Result<Self, Error> {
        todo!()
    }
}

#[derive(Debug, Deserialize)]
pub struct WatchlistCreateOrDeleteRequest {
    group_id: i32,
    asset_id: i32,
}

#[instrument]
async fn check_exists(db: &Arc<dyn Database>, table_name: &str, id: i32) -> Result<bool, ApiError> {
    let query = format!("SELECT EXISTS(SELECT 1 FROM {} WHERE id = $1)", table_name);
    let mut args = PgArguments::default();
    args.add(id);
    let result = db
        .fetch_one(&query, args)
        .await?;

    if result.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

#[instrument]
pub async fn create_watchlist(
    state: Data<AppState>,
    body: Json<WatchlistCreateOrDeleteRequest>
) -> Result<HttpResponse, ApiError> {
    // Check if the group_id exists
    let group_exists = check_exists(&state.db, "watchlist_groups", body.group_id).await?;
    if !group_exists {
        return Err(BadRequest("Watchlist Group not found".into()));
    }

    // Check if the asset_id exists
    let asset_exists = check_exists(&state.db, "assets", body.asset_id).await?;
    if !asset_exists {
        return Err(BadRequest("Asset not found".into()));
    }

    let mut args = PgArguments::default();
    args.add(body.group_id);
    args.add(body.asset_id);

    let record = state.db
        .execute("INSERT INTO watchlist (group_id, asset_id) VALUES ($1, $2)", args)
        .await?;

    // TODO: Add unprocessible entity instead of internal server error.
    if record.rows_affected() == 0 {
        return Err(InternalServerError);
    }

    state.redis_client.del(format!("all_watchlist::{}", body.group_id)).await.expect("Failed to delete a key on Redis");

    respond_ok()
}

#[instrument]
pub async fn retrieve_all_watchlist(
    state: Data<AppState>,
    path: Path<i32>
) -> Result<Json<Vec<WatchlistResponse>>, ApiError> {
    let watchlistgroup_id = path.into_inner();
    let mut args = PgArguments::default();
    let mut watchlist: Vec<WatchlistResponse> = vec![];
    args.add(watchlistgroup_id);

    let cached_data: Result<Vec<WatchlistResponse>, ApiError> = state.redis_client.get(format!("all_watchlist::{}", watchlistgroup_id)).await;

    match cached_data {
        Ok(cached_data) => {
            Ok(respond_json(cached_data).unwrap())
        }
        Err(ApiError::RedisNil) => {
            let records = state.db
                .fetch_all("SELECT a.id, a.name, a.symbol FROM watchlist w JOIN assets a ON w.asset_id = a.id where w.group_id = $1", args)
                .await?;

            if records.is_empty() {
                return respond_json(watchlist);
            }

            watchlist = records
                .iter()
                .map(|record| WatchlistResponse {
                    id: record.get("id"),
                    name: record.get("name"),
                    symbol: record.get("symbol"),
                }).collect();

            state.redis_client.set(format!("all_watchlist::{}", watchlistgroup_id), watchlist.clone()).await.expect("Failed to set the data to Redis");
            respond_json(watchlist)
        }
        _ => Err(InternalServerError)
    }
}

#[instrument]
pub async fn delete_watchlist(
    state: Data<AppState>,
    body: Json<WatchlistCreateOrDeleteRequest>,
    request: HttpRequest
) -> Result<HttpResponse, ApiError> {
    let user_id = request.extensions().get::<Claims>().unwrap().user_id;
    // Check if data is existed
    let mut select_args = PgArguments::default();
    select_args.add(&body.group_id);
    select_args.add(user_id);

    let record = state.db
        .fetch_one("SELECT w.asset_id FROM watchlist w
    JOIN watchlist_groups wg ON w.group_id = wg.id
    JOIN users u ON wg.user_id = u.id WHERE wg.id = $1 AND wg.user_id = $2", select_args)
        .await?;

    if record.is_empty() {
        return Err(BadRequest("Watchlist not found".into()));
    }

    let mut args = PgArguments::default();
    args.add(&body.asset_id);
    args.add(&body.group_id);

    let record = state.db
        .execute("DELETE FROM watchlist WHERE asset_id = $1 and group_id = $2", args)
        .await?;

    if record.rows_affected() == 0 {
        return Err(InternalServerError);
    }

    state.redis_client.del(format!("all_watchlist::{}", body.group_id)).await.expect("Failed to delete a key on Redis");

    respond_ok()
}
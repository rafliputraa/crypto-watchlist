use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use actix_web::web::{Data, Json, Path, Query};
use log::debug;
use redis_async::error::Error;
use redis_async::resp::{FromResp, RespValue};
use sqlx::{Arguments, Row};
use sqlx::postgres::PgArguments;
use tracing::instrument;
use crate::database::Database;
use crate::errors::ApiError;
use crate::errors::ApiError::InternalServerError;
use crate::helpers::{format_datetime, respond_json, respond_ok};
use crate::middleware_custom::Claims;
use crate::server::AppState;
use crate::watchlist::WatchlistResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WatchlistGroupResponse {
    id: i32,
    user_id: i32,
    name: String,
    created_at: String,
}

impl FromResp for WatchlistGroupResponse {
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
pub struct WatchlistGroupCreateOrUpdateRequest {
    name: String,
}

#[instrument]
pub async fn retrieve_all_watchlist_groups(
    state: Data<AppState>,
    request: HttpRequest,
)
    -> Result<Json<Vec<WatchlistGroupResponse>>, ApiError> {

    let user_id = request.extensions().get::<Claims>().unwrap().user_id;
    let mut args = PgArguments::default();
    args.add(user_id);
    let cached_data: Result<Vec<WatchlistGroupResponse>, ApiError> = state.redis_client.get(format!("all_watchlist_group::{}", user_id)).await;

    match cached_data {
        Ok(cached_data) => {
            Ok(respond_json(cached_data).unwrap())
        }
        Err(ApiError::RedisNil) => {
            let records = state.db
                .fetch_all("SELECT * FROM watchlist_groups WHERE user_id = $1", args)
                .await?;
            let watchlist_groups: Vec<WatchlistGroupResponse> = records
                .iter()
                .map(|record| WatchlistGroupResponse {
                    id: record.get("id"),
                    user_id,
                    name: record.get("name"),
                    created_at: format_datetime(record.get("created_at")),
                })
                .collect();
            state.redis_client.set(format!("all_watchlist_group::{}", user_id), watchlist_groups.clone()).await.expect("Failed to set the data to Redis");
            respond_json(watchlist_groups)
        }
        _ => Err(InternalServerError)
    }
}

#[instrument]
pub async fn create_watchlist_group(
    state: Data<AppState>,
    body: Json<WatchlistGroupCreateOrUpdateRequest>,
    request: HttpRequest
) -> Result<Json<WatchlistGroupResponse>, ApiError> {
    let user_id = request.extensions().get::<Claims>().unwrap().user_id;
    let mut args = PgArguments::default();
    args.add(user_id);
    args.add(&body.name);

    let record = state.db
        .fetch_one("INSERT INTO watchlist_groups (user_id, name) VALUES ($1, $2) RETURNING id, created_at", args)
        .await?;

    let watchlist_group = WatchlistGroupResponse {
        id: record.get("id"),
        user_id,
        name: body.name.clone(),
        created_at: format_datetime(record.get("created_at")),
    };

    state.redis_client.del(format!("all_watchlist_group::{}", user_id)).await.expect("Failed to delete a key on Redis");

    respond_json(watchlist_group)
}

#[instrument]
pub async fn update_watchlist_group(
    state: Data<AppState>,
    body: Json<WatchlistGroupCreateOrUpdateRequest>,
    request: HttpRequest,
    path: Path<i32>
)
    -> Result<Json<WatchlistGroupResponse>, ApiError> {
    let user_id = request.extensions().get::<Claims>().unwrap().user_id;
    let group_id = path.into_inner();
    let mut args = PgArguments::default();
    args.add(&body.name);
    args.add(user_id);
    args.add(group_id);

    let record = state.db
        .fetch_one("UPDATE watchlist_groups SET name = COALESCE($1, name) WHERE user_id = $2 AND id = $3 RETURNING name, created_at", args)
        .await?;
    let watchlist_group = WatchlistGroupResponse {
        id: group_id,
        user_id,
        name: record.get("name"),
        created_at: format_datetime(record.get("created_at")),
    };

    state.redis_client.del(format!("all_watchlist_group::{}", user_id)).await.expect("Failed to delete a key on Redis");

    respond_json(watchlist_group)
}

#[instrument]
pub async fn delete_watchlist_group(
    state: Data<AppState>,
    request: HttpRequest,
    path: Path<i32>
) -> Result<HttpResponse, ApiError> {
    let user_id = request.extensions().get::<Claims>().unwrap().user_id;
    let group_id = path.into_inner();
    let mut args = PgArguments::default();
    args.add(group_id);
    args.add(user_id);

    let record = state.db
        .execute("DELETE FROM watchlist_groups WHERE id = $1 AND user_id = $2", args)
        .await?;

    if record.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    state.redis_client.del(format!("all_watchlist_group::{}", user_id)).await.expect("Failed to delete a key on Redis");
    state.redis_client.del(format!("all_watchlist::{}", group_id)).await.expect("Failed to delete a key on Redis");

    respond_ok()
}
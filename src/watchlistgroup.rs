use actix_web::HttpResponse;
use actix_web::web::{Data, Json, Path, Query};
use chrono::NaiveDateTime;
use log::debug;
use crate::errors::ApiError;
use crate::helpers::{format_datetime, respond_json, respond_ok};
use crate::server::AppState;

#[derive(Debug, Serialize)]
pub struct WatchlistGroupResponse {
    id: i32,
    user_id: i32,
    name: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct WatchlistGroupUpdateRequest {
    id: i32,
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct WatchlistGroupCreateRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct WatchlistGroupDeleteRequest {
    id: i32,
}

pub async fn retrieve_all_watchlist_groups(
    state: Data<AppState>,
    path: Path<i32>
)
    -> Result<Json<Vec<WatchlistGroupResponse>>, ApiError> {
    let user_id = path.into_inner();
    let records = sqlx::query!(
        "SELECT * FROM watchlist_groups WHERE user_id = $1",
        user_id)
        .fetch_all(&state.db).await?;
    let mut response = Vec::new();
    for record in records.iter() {
        let watchlist_group = WatchlistGroupResponse {
            id: record.id,
            user_id,
            name: record.name.clone(),
            created_at: format_datetime(record.created_at),
        };
        response.push(watchlist_group)
    };

    respond_json(response)
}

pub async fn create_watchlist_group(
    state: Data<AppState>,
    path: Path<i32>,
    body: Json<WatchlistGroupCreateRequest>
) -> Result<Json<WatchlistGroupResponse>, ApiError> {
    let user_id = path.into_inner();

    let result = sqlx::query!(
        "INSERT INTO watchlist_groups (user_id, name) VALUES ($1, $2) RETURNING id, created_at",
        user_id, body.name
    )
        .fetch_one(&state.db).await?;

    let response = WatchlistGroupResponse {
        id: result.id,
        user_id,
        name: body.name.clone(),
        created_at: format_datetime(result.created_at),
    };

    respond_json(response)
}

pub async fn update_watchlist_group(
    state: Data<AppState>,
    path: Path<i32>,
    body: Json<WatchlistGroupUpdateRequest>
)
    -> Result<Json<WatchlistGroupResponse>, ApiError> {
    let user_id = path.into_inner();
    let record = sqlx::query!(
        "UPDATE watchlist_groups SET name = COALESCE($1, name) WHERE user_id = $2 AND id = $3 RETURNING name, created_at",
        body.name, user_id, body.id)
        .fetch_one(&state.db).await?;
    let watchlist_group = WatchlistGroupResponse {
        id: body.id,
        user_id,
        name: record.name,
        created_at: format_datetime(record.created_at),
    };

    respond_json(watchlist_group)
}

pub async fn delete_watchlist_group(
    state: Data<AppState>,
    path: Path<i32>,
    query: Query<WatchlistGroupDeleteRequest>
) -> Result<HttpResponse, ApiError> {
    let user_id = path.into_inner();
    let group_id = query.into_inner();

    let result = sqlx::query!(
        "DELETE FROM watchlist_groups WHERE id = $1 AND user_id = $2",
        group_id.id, user_id
    )
        .execute(&state.db).await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    respond_ok()
}
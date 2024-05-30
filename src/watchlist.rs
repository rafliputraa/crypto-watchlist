use std::sync::Arc;
use actix_web::HttpResponse;
use actix_web::web::{Data, Json, Path};
use sqlx::{Arguments, Error, Executor, PgPool, Postgres, Row, Transaction};
use sqlx::postgres::any::AnyConnectionBackend;
use sqlx::postgres::PgArguments;
use crate::database::{Database, PostgresDB};
use crate::errors::ApiError;
use crate::helpers::{format_datetime, respond_json, respond_ok};
use crate::server::AppState;
use crate::watchlistgroup::{WatchlistGroupCreateRequest, WatchlistGroupResponse};

#[derive(Debug, Serialize)]
pub struct WatchlistResponse {
    id: i32,
    name: String,
    symbol: String,
}

#[derive(Debug, Deserialize)]
pub struct WatchlistRetrieveRequest {
    group_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct WatchlistCreateRequest {
    group_id: i32,
    asset_id: i32,
}

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

pub async fn create_watchlist(
    state: Data<AppState>,
    body: Json<WatchlistCreateRequest>
) -> Result<HttpResponse, ApiError> {
    // Check if the group_id exists
    let group_exists = check_exists(&state.db, "watchlist_groups", body.group_id).await?;
    if !group_exists {
        return Err(ApiError::BadRequest("Watchlist Group not found".into()));
    }

    // Check if the asset_id exists
    let asset_exists = check_exists(&state.db, "assets", body.asset_id).await?;
    if !asset_exists {
        return Err(ApiError::BadRequest("Asset not found".into()));
    }

    let mut args = PgArguments::default();
    args.add(body.group_id);
    args.add(body.asset_id);

    let record = state.db
        .execute("INSERT INTO watchlist (group_id, asset_id) VALUES ($1, $2)", args)
        .await?;

    // TODO: Add unprocessible entity instead of internal server error.
    if record.rows_affected() == 0 {
        return Err(ApiError::InternalServerError);
    }

    respond_ok()
}

// TODO: Add more handler to get and delete watchlist.
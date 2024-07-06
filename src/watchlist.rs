use std::fmt;
use std::sync::Arc;
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use actix_web::web::{Data, Json, Path};
use derive_more::Display;
use sqlx::{Arguments, Executor, Row};
use sqlx::postgres::PgArguments;
use crate::database::Database;
use crate::errors::ApiError;
use crate::errors::ApiError::{BadRequest, InternalServerError};
use crate::helpers::{respond_json, respond_ok};
use crate::middleware_custom::Claims;
use crate::server::AppState;

#[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize)]
pub struct WatchlistCreateOrDeleteRequest {
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
    body: Json<WatchlistCreateOrDeleteRequest>
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

pub async fn retrieve_all_watchlist(
    state: Data<AppState>,
    path: Path<i32>
) -> Result<Json<Vec<WatchlistResponse>>, ApiError> {
    let watchlistgroup_id = path.into_inner();
    let mut args = PgArguments::default();
    let mut watchlist: Vec<WatchlistResponse> = vec![];
    args.add(watchlistgroup_id);

    state.logging_chan_tx.send(format!("INPUT - retrieve_all_watchlist - watchlistgroup_id:  {}", watchlistgroup_id)).unwrap();

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

    state.logging_chan_tx.send(format!("OUTPUT - retrieve_all_watchlist - watchlist:  {:?}", watchlist)).unwrap();

    respond_json(watchlist)
}

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

    respond_ok()
}
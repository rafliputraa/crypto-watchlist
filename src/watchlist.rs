use std::sync::Arc;
use actix_web::web::{Data, Json, Path};
use sqlx::{Arguments, Error, Executor, PgPool, Postgres, Row, Transaction};
use sqlx::postgres::any::AnyConnectionBackend;
use sqlx::postgres::PgArguments;
use crate::database::{Database, PostgresDB};
use crate::errors::ApiError;
use crate::helpers::{format_datetime, respond_json};
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

async fn check_exists(db: &Arc<dyn Database>, table_name: &str, id: i32) -> bool {
    let mut args = PgArguments::default();
    args.add(table_name);
    args.add(id);
    let result = db
        .fetch_one("SELECT EXISTS(SELECT 1 FROM $1 WHERE id = $2)", args)
        .await?;

    if !result {
        return false;
    }
    true
}

pub async fn create_watchlist(
    state: Data<AppState>,
    body: Json<WatchlistCreateRequest>
) -> Result<Json<WatchlistResponse>, ApiError> {
    let create_watchlist_txn: Transaction<'_, Postgres> = state.db.begin_transaction().await?;
    // Check if the group_id exists
    let group_exists = check_exists(&state.db, "watchlist_groups", body.group_id).await?;
    if !group_exists {
        state.db.rollback_transaction(create_watchlist_txn).await?;
        return Err(ApiError::BadRequest("Watchlist Group not found".into()));
    }

    // // Check if the asset_id exists
    let asset_exists = check_exists(&state.db, "assets", body.asset_id).await?;
    if !asset_exists {
        state.db.rollback_transaction(create_watchlist_txn).await?;
        return Err(ApiError::BadRequest("Asset not found".into()));
    }

    let mut args = PgArguments::default();
    args.add(body.group_id);
    args.add(body.asset_id);

    let record = state.db
        .fetch_one("INSERT INTO watchlist (group_id, asset_id) VALUES ($1, $2) RETURNING id, created_at", args)
        .await?;

    let watchlist = WatchlistResponse {
        id: 1,
        symbol: "BMRI",
        name: "Bank Mandiri",
    };

    respond_json(watchlist)
}
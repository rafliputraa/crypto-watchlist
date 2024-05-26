use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use log::{debug, error};
use reqwest::ClientBuilder;
use crate::config::CONFIG;
use sqlx::{Arguments, Pool, Postgres};
use sqlx::postgres::PgArguments;
use crate::database::{Database, PostgresDB};

#[derive(Debug, Deserialize)]
pub struct CMCAPIResponse {
    status: Status,
    data: Vec<TokenInfo>
}

#[derive(Debug, Deserialize)]
pub struct Status {
    timestamp: String,
    error_code: u32,
    error_message: Option<String>,
    elapsed: u32,
    credit_count: u8,
    notice: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenInfo {
    id: i32,
    rank: i32,
    name: String,
    symbol: String,
    slug: String,
    first_historical_data: String,
    last_historical_data: String,
    platform: Option<Platform>,
}

#[derive(Debug, Deserialize)]
pub struct Platform {
    id: u32,
    name: String,
    symbol: String,
    slug: String,
    tokenAddress: String,
}

pub async fn feed_assets_data(db_conn: Arc<PostgresDB>) -> Result<(), Box<dyn Error>> {
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(20))
        .build().unwrap();

    let response = match client.get(&CONFIG.cmc_token_id_endpoint)
        .header("X-CMC_PRO_API_KEY", &CONFIG.cmc_api_key as &str)
        .send().await {
        Ok(resp) => resp,
        Err(err) => {
            error!("Error making request: {}", err);
            return Err(err.into());
        }
    };

    if !response.status().is_success() {
        return Err(format!("Request failed with status code: {}", response.status()).into());
    }

    // Read the response body as bytes
    let bytes = response.bytes().await?;

    // Convert the bytes to a string (assuming UTF-8 encoding)
    let response_str = String::from_utf8_lossy(&bytes);

    // Log the raw JSON response for debugging
    debug!("Raw JSON response: {}", response_str);

    // Deserialize the response into your data structure
    let api_response: CMCAPIResponse = serde_json::from_slice(&bytes)?;

    // Insert the TokenInfo data into PostgreSQL
    for asset in api_response.data {
        debug!("The symbol: {}", &asset.symbol);
        let first_historical_data = &asset.first_historical_data.parse::<DateTime<Utc>>()?;
        let last_historical_data = &asset.last_historical_data.parse::<DateTime<Utc>>()?;
        let mut args = PgArguments::default();
        args.add(&asset.id);
        args.add(&asset.name);
        args.add(&asset.symbol);
        args.add(&asset.slug);
        args.add(first_historical_data);
        args.add(last_historical_data);

        db_conn
            .execute(r#"INSERT INTO assets (id, name, symbol, slug, first_historical_data, last_historical_data)
                        VALUES ($1, $2, $3, $4, $5, $6)
                        ON CONFLICT (id) DO NOTHING"#, args)
            .await?;
    }

    Ok(())
}
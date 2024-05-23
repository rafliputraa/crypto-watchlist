use std::error::Error;
use std::{fs, io, time};
use std::fs::File;
use std::io::copy;
use std::str::FromStr;
use std::time::Duration;
use chrono::{DateTime, TimeZone, Utc, ParseError};
use log::{debug, error};
use reqwest::{Client, ClientBuilder};
use crate::config::CONFIG;
use futures_util::StreamExt;
use sqlx::{Executor, Pool, Postgres};

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
    id: u32,
    rank: u32,
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

pub async fn retrieve_data(db_conn: Pool<Postgres>) -> Result<(), Box<dyn Error>> {
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
    for token in api_response.data {
        debug!("The symbol: {}", &token.symbol);
        let first_historical_data = DateTime::<Utc>::from_str(&token.first_historical_data)
            .expect("Failed to parse time");
        let last_historical_data = DateTime::<Utc>::from_str(&token.last_historical_data)
            .expect("Failed to parse time");
        sqlx::query(
            r#"INSERT INTO assets (id, name, symbol, slug, first_historical_data, last_historical_data)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (id) DO NOTHING"#)
            .bind(token.id as i32)
            .bind(&token.name)
            .bind(&token.symbol)
            .bind(&token.slug)
            .bind(first_historical_data.to_rfc3339())
            .bind(last_historical_data.to_rfc3339())
            .execute(&db_conn)
            .await?;
    }

    Ok(())
}

pub async fn feed_data() -> Result<(), Box<dyn Error>> {

    Ok(())
}
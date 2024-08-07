mod server;
mod database;
mod config;
mod data_provider;
mod routes;
mod health;
mod errors;
mod helpers;
mod watchlist;
mod watchlistgroup;
mod middleware_custom;
mod cache;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

use server::server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    server().await
}

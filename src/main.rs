mod server;
mod database;
mod config;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

use server::server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    server().await
}

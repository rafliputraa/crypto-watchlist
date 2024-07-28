use actix_web::{App, HttpServer, middleware, web};
use dotenv::dotenv;
use env_logger::Builder;
use log::{debug, error, info};
use sqlx::{Pool, Postgres};
use crate::config::CONFIG;
use crate::database::{create_pool, Database, PostgresDB};
use std::io::Write;
use std::sync::{Arc, mpsc};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use kafka::client::RequiredAcks;
use kafka::producer::{Compression, Producer, Record};
use tracing_actix_web::root_span_macro::private::tracing;
use tracing_actix_web::TracingLogger;
use tracing_appender::rolling;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};
use crate::data_provider::feed_assets_data;
use crate::routes::routes;
use crate::middleware_custom;

#[derive(Debug)]
pub struct AppState {
    pub db: Arc<dyn Database>,
}

pub async fn server() -> std::io::Result<()> {

    dotenv().ok();

    // Build the log format
    LogTracer::init().expect("Unable to setup the log tracer!");
    let app_name = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION")).to_string();
    let file_appender = rolling::daily(&CONFIG.log_file_location, env!("CARGO_PKG_NAME"));
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);
    let bunyan_formatting_layer = BunyanFormattingLayer::new(app_name, non_blocking_writer);
    let subscriber = Registry::default()
        .with(EnvFilter::from_env("LOG_LEVEL"))
        .with(JsonStorageLayer)
        .with(bunyan_formatting_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let pool;
    match create_pool().await {
        Ok(conn) => {
            pool = conn;
        }
        Err(err) => {
            error!("Failed to create database pool: {}", err);
            std::process::exit(1);
        }
    }

    if CONFIG.is_feed_assets_data_enabled {
        match feed_assets_data(pool.clone()).await {
            Ok(()) => debug!("data has been fed to db successfully."),
            Err(err) => {
                error!("There is an error when trying to feed the data to db: {}", err);
                std::process::exit(1);
            }
        }
    }

    info!("🚀 Server started successfully");
    // Start the server
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(middleware_custom::JWTMiddleware::new(CONFIG.jwt_secret.to_string()))
            .app_data(web::Data::new(AppState{
                db: pool.clone(),
            }))
            .configure(routes)
    });
    server.bind(&CONFIG.server)?.run().await
}
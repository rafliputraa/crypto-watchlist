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
use crate::data_provider::feed_assets_data;
use crate::routes::routes;
use crate::middleware_custom;

pub struct AppState {
    pub db: Arc<dyn Database>,
    pub logging_chan_tx: Sender<String>,
}

pub async fn server() -> std::io::Result<()> {

    dotenv().ok();

    // Build the log format
    Builder::from_env(env_logger::Env::default().default_filter_or(&CONFIG.log_level))
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {}] {}",
                record.level(),
                chrono::Local::now().format("%Y-%m-%d - %H:%M:%S").to_string(),
                record.args()
            )
        })
        .init();

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

    let (sender, receiver) = mpsc::channel::<String>();

    start_logging_producer(receiver).await;

    info!("🚀 Server started successfully");
    // Start the server
    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware_custom::JWTMiddleware::new(CONFIG.jwt_secret.to_string()))
            .app_data(web::Data::new(AppState{
                db: pool.clone(),
                logging_chan_tx: sender.clone(),
            }))
            .configure(routes)
    });
    server.bind(&CONFIG.server)?.run().await
}

async fn start_logging_producer(receiver: Receiver<String>) {
    thread::spawn(move || {
        let mut producer_raw = Producer::from_hosts(CONFIG.redpanda_brokers.to_owned())
            .with_ack_timeout( Duration::from_millis(100))
            .with_compression(Compression::SNAPPY)
            .with_required_acks(RequiredAcks::One)
            .create();
        match producer_raw {
            Ok(mut producer) => {
                for received in receiver {
                    debug!("Received log: {:?}", received);
                    producer.send(&Record::from_value(&CONFIG.log_producer_topic, received)).unwrap();
                }
            }
            Err(kafka::Error::NoHostReachable) => {
                error!("No host is reachable");
                std::process::exit(1);
            }
            Err(e) => {
                error!("Unmapped error {}", e);
                std::process::exit(1);
            }
        }
    });

}
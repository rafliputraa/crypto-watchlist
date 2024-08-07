use dotenv::dotenv;

#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    pub database_url: String,
    pub server: String,
    pub app_version: String,
    pub log_level: String,
    pub redis_host: String,
    pub redis_port: u16,
    pub redis_password: String,
    pub cmc_api_key: String,
    pub cmc_token_id_endpoint: String,
    pub is_feed_assets_data_enabled: bool,
    pub jwt_secret: String,
    pub log_file_location: String,
}

lazy_static! {
    pub static ref CONFIG: Config = get_config();
}

fn get_config() -> Config {
    dotenv().ok();

    match envy::from_env::<Config>() {
        Ok(config) => config,
        Err(error) => panic!("Configuration Error: {:#?}", error)
    }
}
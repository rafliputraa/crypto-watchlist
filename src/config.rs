use dotenv::dotenv;

#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    pub database_url: String,
    pub server: String,
    pub app_version: String,
    pub log_level: String,
    pub redis_url: String,
    pub cmc_api_key: String,
    pub cmc_token_id_endpoint: String,
    pub is_feed_assets_data_enabled: bool,
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
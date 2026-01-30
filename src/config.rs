use dotenv::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub app_id: String,
    pub client_secret: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok();

        Self {
            app_id: env::var("QQ_APP_ID").unwrap_or_else(|_| "102640909".to_string()),
            client_secret: env::var("QQ_CLIENT_SECRET")
                .unwrap_or_else(|_| "qU9oUArYGyhQAvgSE1ocQF4ulcUMF82w".to_string()),
        }
    }
}

pub fn get_config() -> Config {
    Config::from_env()
}

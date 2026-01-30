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
            app_id: env::var("QQ_APP_ID").expect("QQ_APP_ID 环境变量未设置"),
            client_secret: env::var("QQ_CLIENT_SECRET").expect("QQ_CLIENT_SECRET 环境变量未设置"),
        }
    }
}

pub fn get_config() -> Config {
    Config::from_env()
}

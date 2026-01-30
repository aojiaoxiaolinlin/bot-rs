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
            app_id: env::var("QQ_APP_ID").expect("需要设置环境变量QQ_APP_ID"),
            client_secret: env::var("QQ_CLIENT_SECRET").expect("需要设置环境变量QQ_CLIENT_SECRET"),
        }
    }
}

pub fn get_config() -> Config {
    Config::from_env()
}

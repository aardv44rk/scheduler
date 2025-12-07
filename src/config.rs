use crate::errors::AppError;
use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub db_url: String,
    pub server_port: u16,
    pub rust_log: String,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        dotenv().ok();

        let db_url = env::var("DATABASE_URL").unwrap_or("sqlite:./scheduler.db".to_string());

        let server_port = match env::var("SERVER_PORT") {
            Ok(port_str) => port_str.parse::<u16>().map_err(|_| {
                AppError::Config(format!(
                    "SERVER_PORT '{}' is not a valid port number",
                    port_str
                ))
            })?,
            Err(_) => 8080, // Default
        };

        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        Ok(Config {
            db_url,
            server_port,
            rust_log,
        })
    }
}

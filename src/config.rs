use serde::Deserialize;
use std::fs;
use crate::error::{Result, ScraperError};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub ticketmaster: TicketmasterConfig,
}

#[derive(Debug, Deserialize)]
pub struct TicketmasterConfig {
    pub delay_ms: u64,
    pub geo_point: String,
    pub timeout_seconds: u64,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = "config.toml";
        let config_content = fs::read_to_string(config_path)
            .map_err(|e| ScraperError::Config(format!("Failed to read config file '{}': {}", config_path, e)))?;
        
        let config: Config = toml::from_str(&config_content)?;
        Ok(config)
    }
}

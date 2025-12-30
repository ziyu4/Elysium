//! Configuration module for Elysium bot.
//!
//! Loads configuration from environment variables.

use serde::Deserialize;
use std::env;

/// Bot running mode
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BotMode {
    Polling,
    Webhook,
}

impl Default for BotMode {
    fn default() -> Self {
        Self::Polling
    }
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    // Telegram
    pub bot_token: String,
    pub bot_mode: BotMode,
    pub webhook_url: Option<String>,

    /// Bot username (without @) for deep link construction.
    /// Optional - will be fetched via getMe if not set.
    pub bot_username: Option<String>,

    /// Owner user IDs (comma-separated)
    /// These users have full access to all bot features.
    pub owner_ids: Vec<u64>,

    // MongoDB
    pub mongodb_uri: String,
    pub mongodb_database: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Panics
    /// Panics if required environment variables are not set.
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let bot_mode = env::var("BOT_MODE")
            .unwrap_or_else(|_| "polling".to_string())
            .to_lowercase();

        let bot_mode = match bot_mode.as_str() {
            "webhook" => BotMode::Webhook,
            _ => BotMode::Polling,
        };

        let webhook_url = env::var("WEBHOOK_URL").ok();

        // Validate webhook URL is set if mode is webhook
        if bot_mode == BotMode::Webhook && webhook_url.is_none() {
            panic!("WEBHOOK_URL must be set when BOT_MODE is webhook");
        }

        // Parse owner IDs
        let owner_ids = env::var("OWNER_IDS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<u64>().ok())
            .collect();

        // Parse bot username (strip @ if present)
        let bot_username = env::var("BOT_USERNAME")
            .ok()
            .map(|s| s.trim_start_matches('@').to_string())
            .filter(|s| !s.is_empty());

        Self {
            bot_token: env::var("BOT_TOKEN").expect("BOT_TOKEN must be set"),
            bot_mode,
            webhook_url,
            bot_username,
            owner_ids,
            mongodb_uri: env::var("MONGODB_URI").expect("MONGODB_URI must be set"),
            mongodb_database: env::var("MONGODB_DATABASE")
                .unwrap_or_else(|_| "elysium".to_string()),
        }
    }
}

//! Elysium - Modular Telegram Bot Framework
//!
//! A super modular Telegram bot for group management.
//!
//! ## Architecture
//!
//! - `config` - Environment configuration
//! - `database` - MongoDB integration
//! - `cache` - LRU-based caching with Moka
//! - `permissions` - Admin checking with caching
//! - `bot` - Core bot functionality (with Throttle for API rate limiting)
//! - `plugins` - Command handlers (extensible)
//! - `events` - Event handlers (extensible)
//! - `utils` - Utility functions

mod bot;
mod cache;
mod config;
mod database;
mod events;
mod permissions;
mod plugins;
mod utils;

use std::sync::Arc;

use teloxide::adaptors::throttle::Limits;
use teloxide::prelude::*;
use tracing::info;
use tracing_subscriber::EnvFilter;

use cache::CacheRegistry;
use config::Config;
use database::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file first (before anything else)
    dotenvy::dotenv().ok();

    // Initialize logging with sensible defaults
    // If RUST_LOG is not set, default to "info" level for our crate
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("elysium=info,teloxide=warn"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    info!("Starting Elysium bot...");

    // Load configuration
    let config = Config::from_env();
    info!("Configuration loaded successfully");
    info!("Bot mode: {:?}", config.bot_mode);

    // Connect to MongoDB
    info!("Connecting to MongoDB...");
    let db = Database::connect(&config.mongodb_uri, &config.mongodb_database).await?;
    let db = Arc::new(db);
    info!("Database connected");

    // Initialize cache registry
    let cache = Arc::new(CacheRegistry::new());
    info!("Cache registry initialized");

    // Initialize bot with Throttle for automatic rate limiting
    // This respects Telegram's rate limits:
    // - 30 messages per second globally
    // - 1 message per second to the same chat
    // - 20 messages per minute to the same group
    let bot = Bot::new(&config.bot_token).throttle(Limits::default());
    info!("Bot initialized with rate limiting (Throttle)");

    // Get bot info
    let me = bot.get_me().await?;
    info!("Bot username: @{}", me.username());

    // Get bot username from config or fallback to get_me()
    let bot_username = config.bot_username.clone()
        .unwrap_or_else(|| me.username().to_string());
    info!("Using bot username: @{}", bot_username);

    // Log owner info
    if config.owner_ids.is_empty() {
        info!("No owner IDs configured (OWNER_IDS is empty)");
    } else {
        info!("Bot owners: {:?}", config.owner_ids);
    }

    // Build dispatcher
    let dispatcher = bot::build_dispatcher(bot, db, cache, config.owner_ids.clone(), bot_username);

    // Run the bot
    bot::run(&config, dispatcher).await;

    Ok(())
}

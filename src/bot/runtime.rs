//! Bot runtime - Polling and Webhook runners.

use teloxide::prelude::*;
use tracing::info;

use super::dispatcher::ThrottledBot;
use super::webhook;
use crate::config::{BotMode, Config};

/// Run the bot with the configured mode.
///
/// Automatically selects between polling and webhook based on config.
pub async fn run(
    config: &Config,
    mut dispatcher: Dispatcher<ThrottledBot, anyhow::Error, teloxide::dispatching::DefaultKey>,
    bot: ThrottledBot,
) {
    match config.bot_mode {
        BotMode::Polling => {
            info!("🔄 Starting bot in polling mode...");
            dispatcher.dispatch().await;
        }
        BotMode::Webhook => {
            info!("🌐 Starting bot in webhook mode...");
            webhook::start_webhook(config, dispatcher, bot).await;
        }
    }
}

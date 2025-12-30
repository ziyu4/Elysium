//! Bot runtime - Polling and Webhook runners.

use teloxide::prelude::*;
use tracing::info;

use super::dispatcher::ThrottledBot;
use crate::config::{BotMode, Config};

/// Run the bot with the configured mode.
///
/// Automatically selects between polling and webhook based on config.
pub async fn run(
    config: &Config,
    mut dispatcher: Dispatcher<ThrottledBot, anyhow::Error, teloxide::dispatching::DefaultKey>,
) {
    match config.bot_mode {
        BotMode::Polling => {
            info!("Starting bot in polling mode...");
            dispatcher.dispatch().await;
        }
        BotMode::Webhook => {
            info!("Starting bot in webhook mode...");
            // Webhook implementation placeholder
            // For production, you'd use axum or actix-web here
            run_webhook(config, dispatcher).await;
        }
    }
}

/// Run the bot with webhook.
///
/// This is a placeholder implementation. For production use,
/// you should integrate with a web framework like axum.
async fn run_webhook(
    config: &Config,
    mut dispatcher: Dispatcher<ThrottledBot, anyhow::Error, teloxide::dispatching::DefaultKey>,
) {
    let webhook_url = config
        .webhook_url
        .as_ref()
        .expect("Webhook URL required for webhook mode");

    info!("Webhook URL: {}", webhook_url);

    // For now, fallback to polling
    // TODO: Implement proper webhook with axum
    info!("Webhook mode not fully implemented, falling back to polling...");
    dispatcher.dispatch().await;
}

//! Webhook mode implementation for the bot.
//!
//! Uses teloxide's built-in axum webhook support to:
//! - Automatically call `setWebhook` on Telegram
//! - Spawn an axum HTTP server to receive updates
//! - Automatically call `deleteWebhook` on shutdown

use std::net::SocketAddr;

use teloxide::prelude::*;
use teloxide::update_listeners::webhooks::{self, Options};
use tracing::info;
use url::Url;

use super::dispatcher::ThrottledBot;
use crate::config::Config;

/// Start the bot in webhook mode.
///
/// This function:
/// 1. Parses the webhook URL from config
/// 2. Configures webhook options (address, URL, secret)
/// 3. Sets up the webhook with Telegram
/// 4. Spawns an axum server to receive updates
/// 5. Dispatches updates through the provided dispatcher
///
/// On shutdown (Ctrl+C), the webhook is automatically deleted.
pub async fn start_webhook(
    config: &Config,
    mut dispatcher: Dispatcher<ThrottledBot, anyhow::Error, teloxide::dispatching::DefaultKey>,
    bot: ThrottledBot,
) {
    // Parse webhook URL from config
    let webhook_url = config
        .webhook_url
        .as_ref()
        .expect("WEBHOOK_URL must be set when using webhook mode");

    let url = Url::parse(webhook_url).expect("Invalid WEBHOOK_URL format");

    // Server address - listen on all interfaces at the configured port
    let address = SocketAddr::from(([0, 0, 0, 0], config.webhook_port));

    // Configure webhook options
    let mut options = Options::new(address, url.clone());

    // Add secret token if configured for additional security
    if let Some(ref secret) = config.webhook_secret {
        options = options.secret_token(secret.clone());
        info!("Webhook secret token configured");
    }

    info!("🔗 Setting webhook URL: {}", url);
    info!("📡 Listening on: {}", address);

    // Create the webhook listener
    // This automatically:
    // 1. Calls setWebhook to register with Telegram
    // 2. Spawns an axum HTTP server
    // 3. Handles incoming updates from Telegram
    // 4. Calls deleteWebhook on shutdown
    //
    // Note: We use bot.inner() to get the underlying Bot without Throttle,
    // because the webhook setup only needs basic API access.
    let listener = webhooks::axum(bot.inner().clone(), options)
        .await
        .expect("Failed to setup webhook");

    info!("✅ Webhook setup complete, waiting for updates...");

    // Create a default error handler that logs errors
    let error_handler = LoggingErrorHandler::with_custom_text("Error from update listener");

    // Dispatch updates using the webhook listener
    dispatcher
        .dispatch_with_listener(listener, error_handler)
        .await;
}

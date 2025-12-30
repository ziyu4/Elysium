//! Event handler system.
//!
//! Add new event handlers by:
//! 1. Creating a new file in this directory
//! 2. Adding `pub mod your_event;` below
//! 3. Adding the handler to `event_handler()`

pub mod antiflood;
pub mod bye;
pub mod filters;
pub mod welcome;

use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;
use tracing::{debug, error};

pub use antiflood::FloodTracker;

// Import handlers from plugins
use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::plugins::afk;

/// Build the combined event handler for chat member updates.
pub fn event_handler() -> UpdateHandler<anyhow::Error> {
    dptree::entry()
        .branch(welcome::handler())
        .branch(bye::handler())
}

/// Build the message event handler.
/// 
/// This runs ALL handlers (antiflood, filters, afk) for each message.
/// Each handler runs independently - one handler's result doesn't stop others.
pub fn message_event_handler() -> UpdateHandler<anyhow::Error> {
    dptree::filter(|msg: Message| {
        // Only process group messages (non-commands are handled individually)
        msg.chat.is_group() || msg.chat.is_supergroup()
    })
    .endpoint(unified_message_handler)
}

/// Unified message handler that runs all sub-handlers.
async fn unified_message_handler(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    flood_tracker: FloodTracker,
) -> anyhow::Result<()> {
    let text = msg.text().unwrap_or("");
    let is_command = text.starts_with('/');
    
    debug!("unified_message_handler: chat={}, text='{}', is_command={}", 
           msg.chat.id, text.chars().take(30).collect::<String>(), is_command);

    // Run antiflood (for non-commands)
    if !is_command {
        if let Err(e) = antiflood::check_antiflood(&bot, &msg, &state, &flood_tracker).await {
            error!("Antiflood error: {}", e);
        }
    }

    // Run filters (for non-commands)
    if !is_command && !text.is_empty() {
        if let Err(e) = filters::check_filters(&bot, &msg, &state).await {
            error!("Filters error: {}", e);
        }
    }

    // Run AFK handler (for all messages - welcome back + reply detection)
    if let Err(e) = afk::afk_handler(bot.clone(), msg.clone(), state.clone()).await {
        error!("AFK handler error: {}", e);
    }

    Ok(())
}


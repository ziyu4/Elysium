//! Ping command plugin.
//!
//! Measures and displays Telegram API latency.

use std::time::Instant;

use teloxide::prelude::*;
use teloxide::types::ReplyParameters;

use crate::bot::dispatcher::{AppState, ThrottledBot};

/// Handle the /ping command - measures Telegram API latency.
pub async fn ping_command(
    bot: ThrottledBot,
    msg: Message,
    _state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    
    // Measure time to call getMe (lightweight API call)
    let start = Instant::now();
    let _ = bot.get_me().await;
    let elapsed = start.elapsed();
    
    let ms = elapsed.as_millis();
    
    // Choose emoji based on latency
    let emoji = if ms < 100 {
        "🟢" // Fast
    } else if ms < 300 {
        "🟡" // Medium  
    } else {
        "🔴" // Slow
    };
    
    let text = format!("{} Pong! <code>{}ms</code>", emoji, ms);
    
    bot.send_message(chat_id, text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    
    Ok(())
}

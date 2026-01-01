//! Antiflood event handler.
//!
//! Monitors messages and applies penalties for flooding.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use teloxide::prelude::*;
use teloxide::types::{ChatPermissions, ParseMode};
use tracing::{debug, info, warn};

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::FloodPenalty;
use crate::utils::{html_escape, format_duration_full};
use crate::i18n::get_text;

/// User's flood tracking data
#[derive(Debug, Clone)]
struct UserFloodData {
    message_times: Vec<Instant>,
    warnings: u32,
}

impl UserFloodData {
    fn new() -> Self {
        Self {
            message_times: Vec::new(),
            warnings: 0,
        }
    }
}

/// Per-chat tracking: last user who spoke
#[derive(Debug, Clone, Default)]
struct ChatFloodState {
    /// User flood data per user in this chat
    users: HashMap<u64, UserFloodData>,
    /// Last user who sent a message (for reset logic)
    last_user_id: Option<u64>,
}

/// Global flood tracker (in-memory, lock-free).
#[derive(Clone)]
pub struct FloodTracker {
    /// Per-chat flood state using DashMap for lock-free access
    data: Arc<DashMap<i64, ChatFloodState>>,
}

impl FloodTracker {
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    /// Record a message and check if user is flooding.
    /// If a different user sends a message, reset all other users' counters in that chat.
    /// Returns (is_flooding, warning_count)
    pub fn record_message(
        &self,
        chat_id: i64,
        user_id: u64,
        max_messages: u32,
        window_secs: u32,
    ) -> (bool, u32) {
        let now = Instant::now();
        let window = Duration::from_secs(window_secs as u64);

        let mut chat_state = self.data.entry(chat_id).or_default();

        // If a different user spoke, reset all other users' counters (conversation interrupt)
        if let Some(last_user) = chat_state.last_user_id
            && last_user != user_id {
                // Different user spoke - reset counters for all users except current
                for (uid, user_data) in chat_state.users.iter_mut() {
                    if *uid != user_id {
                        user_data.message_times.clear();
                        // Keep warnings, only reset message counter
                    }
                }
            }

        // Update last user
        chat_state.last_user_id = Some(user_id);

        // Get or create user flood data
        let entry = chat_state.users.entry(user_id).or_insert_with(UserFloodData::new);

        // Clean old messages outside window
        entry.message_times.retain(|&t| now.duration_since(t) < window);

        // Add current message
        entry.message_times.push(now);

        // Check if flooding
        let is_flooding = entry.message_times.len() > max_messages as usize;

        if is_flooding {
            entry.warnings += 1;
        }

        (is_flooding, entry.warnings)
    }

    /// Reset all data for a user in a chat
    pub fn reset_user(&self, chat_id: i64, user_id: u64) {
        if let Some(mut chat_state) = self.data.get_mut(&chat_id) {
            chat_state.users.remove(&user_id);
        }
    }
}

impl Default for FloodTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if this is a group message (not a command)
fn is_group_message(msg: Message) -> bool {
    // Only process in groups
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return false;
    }

    // Skip commands
    if let Some(text) = msg.text()
        && text.starts_with('/') {
            return false;
        }

    // Skip messages without sender
    if msg.from.is_none() {
        return false;
    }

    true
}

/// Public function to check antiflood - called from unified handler.
pub async fn check_antiflood(
    bot: &ThrottledBot,
    msg: &Message,
    state: &AppState,
    flood_tracker: &FloodTracker,
) -> anyhow::Result<()> {
    // Skip if not group message
    if !is_group_message(msg.clone()) {
        return Ok(());
    }
    
    // Call the internal handler logic
    antiflood_check_impl(bot, msg, state, flood_tracker).await
}



/// Internal antiflood check implementation.
async fn antiflood_check_impl(
    bot: &ThrottledBot,
    msg: &Message,
    state: &AppState,
    flood_tracker: &FloodTracker,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    // Skip bots
    if user.is_bot {
        return Ok(());
    }

    let user_id = user.id;

    // Get message context (lightweight, cached)
    let ctx = state.message_context.get_or_default(chat_id.0).await?;

    // Check if antiflood is enabled
    if !ctx.antiflood.enabled {
        return Ok(());
    }

    // Bot owners bypass all restrictions
    if state.is_owner(user_id.0) {
        debug!("User {} is owner, bypassing antiflood", user_id);
        return Ok(());
    }

    // Check if user is approved (bypass antiflood) - O(n) but small list
    if ctx.is_approved(user_id.0) {
        debug!("User {} is approved, bypassing antiflood", user_id);
        return Ok(());
    }

    // Check if user is admin (admins bypass antiflood)
    if state.permissions.is_admin(chat_id, user_id).await.unwrap_or(false) {
        return Ok(());
    }

    // Resolve locale
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    let (is_flooding, warnings) = flood_tracker.record_message(
        chat_id.0,
        user_id.0,
        ctx.antiflood.max_messages,
        ctx.antiflood.time_window_secs,
    );

    if !is_flooding {
        return Ok(());
    }

    debug!(
        "User {} is flooding in chat {} (warning {})",
        user_id, chat_id, warnings
    );

    // Check if we should apply penalty or just warn
    if warnings <= ctx.antiflood.warnings_before_penalty {
        // Send warning
        let remaining = ctx.antiflood.warnings_before_penalty - warnings + 1;
        
        let warning_msg = get_text(&locale, "antiflood.flood_warning")
            .replace("{id}", &user_id.to_string())
            .replace("{name}", &html_escape(&user.first_name))
            .replace("{count}", &remaining.to_string());

        bot.send_message(chat_id, warning_msg)
            .parse_mode(ParseMode::Html)
            .await?;
        return Ok(());
    }

    info!(
        "Applying flood penalty {:?} to user {} in chat {}",
        ctx.antiflood.penalty, user_id, chat_id
    );

    match ctx.antiflood.penalty {
        FloodPenalty::Warn => {
            bot.send_message(
                chat_id,
                get_text(&locale, "antiflood.penalty_warn_msg")
                    .replace("{id}", &user_id.to_string())
                    .replace("{name}", &html_escape(&user.first_name)),
            )
            .parse_mode(ParseMode::Html)
            .await?;
        }
        FloodPenalty::Mute => {
            let until = if ctx.antiflood.penalty_duration_secs > 0 {
                chrono::Utc::now()
                    + chrono::Duration::seconds(ctx.antiflood.penalty_duration_secs as i64)
            } else {
                // Permanent mute (use a far future date)
                chrono::Utc::now() + chrono::Duration::days(366)
            };

            let perms = ChatPermissions::empty(); // No permissions = muted

            match bot
                .restrict_chat_member(chat_id, user_id, perms)
                .until_date(until)
                .await
            {
                Ok(_) => {
                    let duration_str = format!("selama {}", format_duration_full(ctx.antiflood.penalty_duration_secs));
                    // Ideally format_duration_full should also be localized later, but sticking to English/Simple for now
                    // or we can pass localized "selama 5 menit" if we had localized duration formatter.
                    // For now, let's keep it simple as part of the message replacement or just inject it.
                    // The i18n key expects `{duration}`.
                    
                    bot.send_message(
                        chat_id,
                        get_text(&locale, "antiflood.penalty_mute_msg")
                            .replace("{id}", &user_id.to_string())
                            .replace("{name}", &html_escape(&user.first_name))
                            .replace("{duration}", &duration_str),
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                }
                Err(e) => {
                    warn!("Failed to mute user {}: {}", user_id, e);
                }
            }
        }
        FloodPenalty::Kick => {
            match bot.ban_chat_member(chat_id, user_id).await {
                Ok(_) => {
                    // Unban immediately so they can rejoin
                    let _ = bot.unban_chat_member(chat_id, user_id).await;
                    bot.send_message(
                        chat_id,
                        get_text(&locale, "antiflood.penalty_kick_msg")
                            .replace("{id}", &user_id.to_string())
                            .replace("{name}", &html_escape(&user.first_name)),
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                }
                Err(e) => {
                    warn!("Failed to kick user {}: {}", user_id, e);
                }
            }
        }
        FloodPenalty::TempBan => {
            let until = chrono::Utc::now()
                + chrono::Duration::seconds(ctx.antiflood.penalty_duration_secs as i64);

            match bot
                .ban_chat_member(chat_id, user_id)
                .until_date(until)
                .await
            {
                Ok(_) => {
                    let duration_str = format!("selama {}", format_duration_full(ctx.antiflood.penalty_duration_secs));
                    bot.send_message(
                        chat_id,
                        get_text(&locale, "antiflood.penalty_tban_msg")
                            .replace("{id}", &user_id.to_string())
                            .replace("{name}", &html_escape(&user.first_name))
                            .replace("{duration}", &duration_str),
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                }
                Err(e) => {
                    warn!("Failed to tempban user {}: {}", user_id, e);
                }
            }
        }
        FloodPenalty::Ban => {
            match bot.ban_chat_member(chat_id, user_id).await {
                Ok(_) => {
                    bot.send_message(
                        chat_id,
                        get_text(&locale, "antiflood.penalty_ban_msg")
                            .replace("{id}", &user_id.to_string())
                            .replace("{name}", &html_escape(&user.first_name)),
                    )
                    .parse_mode(ParseMode::Html)
                    .await?;
                }
                Err(e) => {
                    warn!("Failed to ban user {}: {}", user_id, e);
                }
            }
        }
    }

    // Reset user's flood tracking after penalty
    flood_tracker.reset_user(chat_id.0, user_id.0);

    Ok(())
}



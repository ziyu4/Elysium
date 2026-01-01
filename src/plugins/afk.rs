//! AFK command handlers.
//!
//! Commands for setting and managing AFK status.
//! Optimized to use CachedUser for zero-latency checks.

use teloxide::prelude::*;
use teloxide::types::{
    MessageId, ParseMode, ReplyParameters, MessageEntityKind,
};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::utils::{format_duration_full, html_escape};
use crate::i18n::get_text;

/// Handle /afk command - set AFK status.
///
/// Usage: /afk [reason]
pub async fn afk_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    // Get reason from command args
    let text = msg.text().unwrap_or("");
    let reason = text
        .split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim().to_string())
        .filter(|s| !s.is_empty());

    // Save AFK status (Updates Cache & DB)
    state.users.set_afk(user_id, reason.clone()).await?;

    info!("User {} went AFK in chat {}", user_id, chat_id);

    let locale = state.get_locale(Some(chat_id.0), Some(user_id)).await;
    
    let reason_text = reason
        .map(|r| get_text(&locale, "afk.reason").replace("{reason}", &html_escape(&r)))
        .unwrap_or_default();

    bot.send_message(
        chat_id,
        get_text(&locale, "afk.now_afk")
            .replace("{id}", &user_id.to_string())
            .replace("{name}", &html_escape(&user.first_name))
            .replace("{reason}", &reason_text),
    )
    .parse_mode(ParseMode::Html)
    .reply_parameters(ReplyParameters::new(msg.id))
    .await?;

    Ok(())
}

/// Handle /brb command - alias for /afk.
pub async fn brb_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    afk_command(bot, msg, state).await
}

/// AFK handler - detect replies/mentions to AFK users and auto-remove AFK.
pub async fn afk_handler(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    // Only process in groups
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    let chat_id = msg.chat.id;
    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };
    let user_id = user.id.0;

    // 1. Check if current user is AFK (Auto-Remove)
    // We check cache first
    let current_user_data = state.users.get_by_id(user_id).await?;
    if let Some(data) = current_user_data {
        if let Some(reason) = &data.afk_reason {
            // User is AFK, remove it
            let duration_secs = data.afk_time.map(|t| chrono::Utc::now().timestamp() - t).unwrap_or(0) as u64;
            let duration = format_duration_full(duration_secs);
            
            state.users.remove_afk(user_id).await?;

            let locale = state.get_locale(Some(chat_id.0), Some(user_id)).await;

            let reason_text = get_text(&locale, "afk.reason")
                .replace("{reason}", &html_escape(reason));

            bot.send_message(
                chat_id,
                get_text(&locale, "afk.returned_afk")
                    .replace("{id}", &user_id.to_string())
                    .replace("{name}", &html_escape(&user.first_name))
                    .replace("{reason}", &reason_text)
                    .replace("{duration}", &duration),
            )
            .parse_mode(ParseMode::Html)
            .disable_notification(true)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        }
    }

    // Track which user IDs we've already notified about (to avoid duplicates)
    let mut notified_users: std::collections::HashSet<u64> = std::collections::HashSet::new();

    // 2. Check Reply to AFK User
    if let Some(reply) = msg.reply_to_message()
        && let Some(reply_user) = &reply.from {
            let reply_user_id = reply_user.id.0;
            // Fetch replied user data
                    if let Ok(Some(target)) = state.users.get_by_id(reply_user_id).await {
                if target.afk_reason.is_some() && !notified_users.contains(&reply_user_id) {
                    send_afk_notification(&bot, chat_id, msg.id, &target, &state).await?;
                    notified_users.insert(reply_user_id);
                }
            }
        }

    // 3. Check Mentions in message text
    let msg_text = msg.text().unwrap_or("");
    
    if let Some(entities) = msg.entities() {
        for entity in entities {
            match &entity.kind {
                // TextMention (Clickable Name)
                MessageEntityKind::TextMention { user: mentioned_user } => {
                    let mentioned_user_id = mentioned_user.id.0;
                    if let Ok(Some(target)) = state.users.get_by_id(mentioned_user_id).await {
                        if target.afk_reason.is_some() && !notified_users.contains(&mentioned_user_id) {
                            send_afk_notification(&bot, chat_id, msg.id, &target, &state).await?;
                            notified_users.insert(mentioned_user_id);
                        }
                    }
                },
                // @username Mention
                MessageEntityKind::Mention => {
                    let start = entity.offset;
                    let end = start + entity.length;
                    if let Some(mention_text) = msg_text.get(start..end) {
                        let username = mention_text.trim_start_matches('@');
                        
                        // Resolve username -> UserData (Includes AFK status!)
                        if let Ok(Some(target)) = state.users.get_by_username(username).await {
                             if target.afk_reason.is_some() && !notified_users.contains(&target.user_id) {
                                send_afk_notification(&bot, chat_id, msg.id, &target, &state).await?;
                                notified_users.insert(target.user_id);
                            }
                        }
                    }
                },
                _ => {}
            }
        }
    }

    Ok(())
}

async fn send_afk_notification(
    bot: &ThrottledBot,
    chat_id: teloxide::types::ChatId,
    reply_to_msg_id: MessageId,
    user: &crate::database::CachedUser,
    state: &AppState,
) -> anyhow::Result<()> {
    let duration_secs = user.afk_time.map(|t| chrono::Utc::now().timestamp() - t).unwrap_or(0) as u64;
    let duration = format_duration_full(duration_secs);
    
    // Resolve locale - we can use chat_id default because we are notifying the group
    let locale = state.get_locale(Some(chat_id.0), None).await;

    let reason_text = user.afk_reason
        .as_ref()
        .map(|r| get_text(&locale, "afk.reason").replace("{reason}", &html_escape(r)))
        .unwrap_or_default();

    bot.send_message(
        chat_id,
        get_text(&locale, "afk.is_afk")
            .replace("{id}", &user.user_id.to_string())
            .replace("{name}", &html_escape(&user.first_name))
            .replace("{reason}", &reason_text)
            .replace("{duration}", &duration),
    )
    .parse_mode(ParseMode::Html)
    .disable_notification(true)
    .reply_parameters(ReplyParameters::new(reply_to_msg_id))
    .await?;

    Ok(())
}

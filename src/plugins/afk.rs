//! AFK command handlers.
//!
//! Commands for setting and managing AFK status.

use teloxide::prelude::*;
use teloxide::types::{
    MessageId, ParseMode, ReplyParameters, MessageEntityKind,
};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::AfkStatus;
use crate::database::GroupSettingsRepo;
use crate::utils::{format_duration_full, html_escape};

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

    // Save AFK status with username for mention detection
    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;
    settings.afk.set_afk(
        user_id,
        reason.clone(),
        user.first_name.clone(),
        user.username.clone(),
    );
    repo.save(&settings).await?;

    info!("User {} went AFK in chat {}", user_id, chat_id);

    let reason_text = reason
        .map(|r| format!("\nAlasan: {}", html_escape(&r)))
        .unwrap_or_default();

    bot.send_message(
        chat_id,
        format!(
            "💤 <a href=\"tg://user?id={}\">{}</a> sekarang AFK!{}",
            user_id,
            html_escape(&user.first_name),
            reason_text
        ),
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

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;

    // Check if current user is AFK - if so, welcome them back
    if settings.afk.is_afk(user_id) {
        if let Some(afk_status) = settings.afk.remove_afk(user_id) {
            repo.save(&settings).await?;

            let duration = format_duration_full(afk_status.duration_secs());
            let reason_text = afk_status
                .reason
                .as_ref()
                .map(|r| format!("\nAlasan: {}", html_escape(r)))
                .unwrap_or_default();

            bot.send_message(
                chat_id,
                format!(
                    "<a href=\"tg://user?id={}\">{}</a> kembali dari afk.{}\nSejak: {} yang lalu.",
                    user_id,
                    html_escape(&user.first_name),
                    reason_text,
                    duration
                ),
            )
            .parse_mode(ParseMode::Html)
            .disable_notification(true)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        }
    }

    // Track which user IDs we've already notified about (to avoid duplicates)
    let mut notified_users: std::collections::HashSet<u64> = std::collections::HashSet::new();

    // Check if replying to an AFK user
    if let Some(reply) = msg.reply_to_message() {
        if let Some(reply_user) = &reply.from {
            let reply_user_id = reply_user.id.0;
            if let Some(afk_status) = settings.afk.get_afk(reply_user_id) {
                if !notified_users.contains(&reply_user_id) {
                    send_afk_notification(&bot, chat_id, msg.id, reply_user_id, afk_status).await?;
                    notified_users.insert(reply_user_id);
                }
            }
        }
    }

    // Check mentions in message text
    let msg_text = msg.text().unwrap_or("");
    
    if let Some(entities) = msg.entities() {
        for entity in entities {
            match &entity.kind {
                // TextMention - when user is mentioned by clicking their name
                MessageEntityKind::TextMention { user: mentioned_user } => {
                    let mentioned_user_id = mentioned_user.id.0;
                    if let Some(afk_status) = settings.afk.get_afk(mentioned_user_id) {
                        if !notified_users.contains(&mentioned_user_id) {
                            send_afk_notification(&bot, chat_id, msg.id, mentioned_user_id, afk_status).await?;
                            notified_users.insert(mentioned_user_id);
                        }
                    }
                },
                // Mention - @username style mention
                MessageEntityKind::Mention => {
                    // Extract the username from the message text
                    let start = entity.offset;
                    let end = start + entity.length;
                    if let Some(mention_text) = msg_text.get(start..end) {
                        // Remove the @ prefix
                        let username = mention_text.trim_start_matches('@');
                        
                        // Look up if this username belongs to an AFK user
                        if let Some((afk_user_id, afk_status)) = settings.afk.get_afk_by_username(username) {
                            if !notified_users.contains(&afk_user_id) {
                                send_afk_notification(&bot, chat_id, msg.id, afk_user_id, afk_status).await?;
                                notified_users.insert(afk_user_id);
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
    user_id: u64,
    afk_status: &AfkStatus,
) -> anyhow::Result<()> {
    let duration = format_duration_full(afk_status.duration_secs());
    let reason_text = afk_status
        .reason
        .as_ref()
        .map(|r| format!("\nAlasan: {}", html_escape(r)))
        .unwrap_or_default();

    bot.send_message(
        chat_id,
        format!(
            "<a href=\"tg://user?id={}\">{}</a> sedang afk.{}\nSejak: {} yang lalu.",
            user_id,
            html_escape(&afk_status.first_name),
            reason_text,
            duration
        ),
    )
    .parse_mode(ParseMode::Html)
    .disable_notification(true)
    .reply_parameters(ReplyParameters::new(reply_to_msg_id))
    .await?;

    Ok(())
}

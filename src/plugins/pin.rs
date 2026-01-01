//! Pin management commands.
//!
//! Commands for pinning and unpinning messages.

use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters, UserId};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::i18n::get_text;

/// Handle /pin command - pin a message.
/// 
/// By default pins silently. Add 'loud' or 'notify' to send notification.
/// Usage: Reply to a message with /pin or /pin loud
pub async fn pin_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    // Must be in group
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, get_text(&locale, "pin.error_group_only"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Check permission: can_pin_messages
    if !state.permissions.can_pin_messages(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanPinMessages"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Check if reply
    let reply = match msg.reply_to_message() {
        Some(r) => r,
        None => {
            bot.send_message(chat_id, get_text(&locale, "pin.error_no_reply"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    // Check for 'loud' or 'notify' argument
    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();
    let notify = args.iter().any(|&a| a.eq_ignore_ascii_case("loud") || a.eq_ignore_ascii_case("notify"));

    // Attempt to pin
    match bot.pin_chat_message(chat_id, reply.id)
        .disable_notification(!notify)
        .await
    {
        Ok(_) => {
            info!("Pinned message {} in chat {} (notify: {})", reply.id, chat_id, notify);
            let text = if notify {
                get_text(&locale, "pin.pinned_notify")
            } else {
                get_text(&locale, "pin.pinned")
            };
            
            bot.send_message(chat_id, text)
                .reply_parameters(ReplyParameters::new(reply.id))
                .await?;
        }
        Err(e) => {
            bot.send_message(chat_id, get_text(&locale, "pin.error_failed").replace("{error}", &e.to_string()))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
    }

    Ok(())
}

/// Handle /permapin command - send and pin a custom message.
///
/// Usage: /permapin <text> - Bot will send the text and pin it.
/// Supports markdown formatting.
pub async fn permapin_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    // Must be in group
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;
        bot.send_message(chat_id, get_text(&locale, "pin.error_group_only"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    // Check permission: can_pin_messages
    if !state.permissions.can_pin_messages(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanPinMessages"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Extract text to pin
    let text = msg.text().unwrap_or("");
    let content = text.strip_prefix("/permapin").unwrap_or("").trim();

    if content.is_empty() {
        bot.send_message(chat_id, get_text(&locale, "pin.permapin_missing_content"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Send the message
    let sent = bot.send_message(chat_id, content)
        .parse_mode(ParseMode::Html)
        .await?;

    // Pin it silently
    match bot.pin_chat_message(chat_id, sent.id)
        .disable_notification(true)
        .await
    {
        Ok(_) => {
            info!("Permapin message {} in chat {}", sent.id, chat_id);
            // Delete the command message
            let _ = bot.delete_message(chat_id, msg.id).await;
        }
        Err(e) => {
            bot.send_message(chat_id, get_text(&locale, "pin.permapin_failed").replace("{error}", &e.to_string()))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
    }

    Ok(())
}

/// Handle /pinned command - get the current pinned message.
pub async fn pinned_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }
    
    let locale = state.get_locale(Some(chat_id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;

    // Get chat info to find pinned message
    match bot.get_chat(chat_id).await {
        Ok(chat) => {
            if let Some(pinned) = &chat.pinned_message {
                // Create link to the pinned message
                let link = if let Some(username) = chat.username() {
                    format!("https://t.me/{}/{}", username, pinned.id)
                } else {
                    // For private groups, use c/ format
                    let chat_id_num = chat_id.0.to_string().replace("-100", "");
                    format!("https://t.me/c/{}/{}", chat_id_num, pinned.id)
                };

                let preview = pinned.text().unwrap_or("[Media/Sticker]");
                let preview_truncated = if preview.len() > 100 {
                    format!("{}...", &preview[..100])
                } else {
                    preview.to_string()
                };

                bot.send_message(
                    chat_id,
                    get_text(&locale, "pin.pinned_header")
                        .replace("{preview}", &crate::utils::html_escape(&preview_truncated))
                        .replace("{link}", &link)
                )
                .parse_mode(ParseMode::Html)
                .link_preview_options(teloxide::types::LinkPreviewOptions {
                    is_disabled: true,
                    url: None,
                    prefer_small_media: false,
                    prefer_large_media: false,
                    show_above_text: false,
                })
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            } else {
                bot.send_message(chat_id, get_text(&locale, "pin.pinned_none"))
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
        }
        Err(e) => {
            bot.send_message(chat_id, format!("❌ Gagal mengambil info chat: {}", e))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
    }

    Ok(())
}

/// Handle /unpin command - unpin a message.
///
/// Usage: Reply to a pinned message or just /unpin to unpin latest.
pub async fn unpin_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    if !state.permissions.can_pin_messages(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanPinMessages"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    let message_id = msg.reply_to_message().map(|m| m.id);

    if let Some(mid) = message_id {
        match bot.unpin_chat_message(chat_id).message_id(mid).await {
            Ok(_) => {
                bot.send_message(chat_id, get_text(&locale, "pin.unpinned"))
                    .reply_parameters(ReplyParameters::new(mid))
                    .await?;
            }
            Err(e) => {
                bot.send_message(chat_id, get_text(&locale, "pin.unpin_failed").replace("{error}", &e.to_string()))
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
        }
    } else {
        // If no reply, unpin the most recent pin
        match bot.unpin_chat_message(chat_id).await {
            Ok(_) => {
                 bot.send_message(chat_id, get_text(&locale, "pin.unpinned_latest"))
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
             Err(e) => {
                bot.send_message(chat_id, get_text(&locale, "pin.unpin_failed").replace("{error}", &e.to_string()))
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
        }
    }

    Ok(())
}

/// Handle /unpinall command - unpin all pinned messages.
pub async fn unpinall_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    if !state.permissions.can_pin_messages(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanPinMessages"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    match bot.unpin_all_chat_messages(chat_id).await {
        Ok(_) => {
            bot.send_message(chat_id, get_text(&locale, "pin.unpin_all_success"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
        Err(e) => {
            bot.send_message(chat_id, get_text(&locale, "pin.unpin_all_failed").replace("{error}", &e.to_string()))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
    }

    Ok(())
}

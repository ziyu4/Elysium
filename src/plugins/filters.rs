//! Filter command handlers.
//!
//! Commands for managing auto-reply filters in groups.

use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters, UserId};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::{DbFilter, MatchType};
use crate::utils::{html_escape, parse_content};
use crate::i18n::get_text;

/// Handle /filter command - add a new filter.
///
/// Usage:
/// - /filter <trigger> <reply>
/// - /filter "multi word trigger" <reply>
/// - /filter (trigger1, trigger2) <reply>
pub async fn filter_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    // Must be in group
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, "⚠️ Perintah ini hanya untuk grup.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Check permission: can_change_info
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;
    if !state.permissions.can_change_info(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanChangeInfo"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args = text.split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim())
        .unwrap_or("");

    if args.is_empty() {
        // Show help - reuse help.filters_text
        bot.send_message(
            chat_id,
            get_text(&locale, "help.filters_text"),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Parse trigger and reply
    let (trigger, reply) = parse_filter_args(args);

    if trigger.is_empty() {
        bot.send_message(chat_id, get_text(&locale, "filters.error_empty_trigger"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Check for reply to media
    let (media_file_id, media_type) = if let Some(reply_msg) = msg.reply_to_message() {
        extract_media(reply_msg)
    } else {
        (None, None)
    };

    // Parse the reply content
    let mut final_reply = reply.clone();
    
    // If no reply text but replying to message, use that message's text
    if final_reply.is_empty()
        && let Some(reply_msg) = msg.reply_to_message() {
            final_reply = reply_msg.text()
                .or_else(|| reply_msg.caption())
                .map(String::from)
                .unwrap_or_default();
        }

    if final_reply.is_empty() && media_file_id.is_none() {
        bot.send_message(chat_id, get_text(&locale, "filters.error_empty_reply"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Determine match type from trigger prefix
    let (clean_trigger, match_type) = parse_trigger_type(&trigger);

    // Parse content for tags and buttons
    let parsed = parse_content(&final_reply);

    // Create filter using DbFilter
    let filter = DbFilter {
        id: None,
        chat_id: chat_id.0,
        trigger: clean_trigger.to_lowercase(),
        match_type,
        reply: parsed.text.clone(),
        buttons: parsed.buttons,
        media_file_id,
        media_type,
        admin_only: parsed.tags.admin_only,
        user_only: parsed.tags.user_only,
        protect: parsed.tags.protect,
        replytag: parsed.tags.replytag,
    };

    // Save filter using FilterRepository
    state.filters.save_filter(&filter).await?;

    info!("Added filter '{}' in chat {}", clean_trigger, chat_id);

    bot.send_message(
        chat_id,
        get_text(&locale, "filters.added")
            .replace("{trigger}", &html_escape(&clean_trigger)),
    )
    .parse_mode(ParseMode::Html)
    .reply_parameters(ReplyParameters::new(msg.id))
    .await?;

    Ok(())
}

/// Handle /filters command - list all filters.
pub async fn filters_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    // Must be in group
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, "⚠️ Perintah ini hanya untuk grup.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Get triggers from FilterRepository (L1 cache)
    let triggers = state.filters.get_triggers(chat_id.0).await?;

    let locale = state.get_locale(Some(chat_id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;

    if triggers.is_empty() {
        bot.send_message(chat_id, get_text(&locale, "filters.none"))
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let mut text = get_text(&locale, "filters.list_header")
        .replace("{count}", &triggers.len().to_string());

    for trigger in triggers {
        text.push_str(&format!("• <code>{}</code>\n", html_escape(&trigger)));
    }
    text.push_str(&get_text(&locale, "filters.list_footer"));

    bot.send_message(chat_id, text)
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

/// Handle /stop command - remove a filter.
pub async fn stop_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    // Must be in group
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, "⚠️ Perintah ini hanya untuk grup.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Check permission
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;
    if !state.permissions.can_change_info(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanChangeInfo"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let trigger = text.split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim())
        .unwrap_or("");

    if trigger.is_empty() {
        bot.send_message(chat_id, get_text(&locale, "filters.stop_usage"))
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Delete filter using FilterRepository
    if state.filters.delete_filter(chat_id.0, trigger).await? {
        info!("Removed filter '{}' from chat {}", trigger, chat_id);

        bot.send_message(
            chat_id,
            get_text(&locale, "filters.deleted")
                .replace("{trigger}", &html_escape(trigger)),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    } else {
        bot.send_message(
            chat_id,
            get_text(&locale, "filters.not_found")
                .replace("{trigger}", &html_escape(trigger)),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    }

    Ok(())
}

/// Handle /stopall command - remove all filters.
pub async fn stopall_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    // Must be in group
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, "⚠️ Perintah ini hanya untuk grup.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Check permission: must be owner
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;
    if !state.permissions.is_owner(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "GroupOwner"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Get current trigger count
    let triggers = state.filters.get_triggers(chat_id.0).await?;
    let count = triggers.len();

    // Delete all filters one by one
    for trigger in triggers {
        let _ = state.filters.delete_filter(chat_id.0, &trigger).await;
    }

    info!("Cleared all {} filters from chat {}", count, chat_id);

    bot.send_message(
        chat_id,
        get_text(&locale, "filters.deleted_all")
            .replace("{count}", &count.to_string()),
    )
    .reply_parameters(ReplyParameters::new(msg.id))
    .await?;

    Ok(())
}

/// Parse filter arguments to extract trigger and reply.
fn parse_filter_args(args: &str) -> (String, String) {
    let args = args.trim();
    
    // Check for quoted trigger: /filter "multi word" reply
    if args.starts_with('"')
        && let Some(end_quote) = args[1..].find('"') {
            let trigger = args[1..end_quote + 1].to_string();
            let reply = args[end_quote + 2..].trim().to_string();
            return (trigger, reply);
        }
    
    // Simple: /filter trigger reply
    let parts: Vec<&str> = args.splitn(2, char::is_whitespace).collect();
    let trigger = parts.first().map(|s| s.to_string()).unwrap_or_default();
    let reply = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
    
    (trigger, reply)
}

/// Parse trigger type from prefix.
fn parse_trigger_type(trigger: &str) -> (String, MatchType) {
    if trigger.starts_with("exact:") {
        (trigger.strip_prefix("exact:").unwrap().to_string(), MatchType::Exact)
    } else if trigger.starts_with("prefix:") {
        (trigger.strip_prefix("prefix:").unwrap().to_string(), MatchType::Prefix)
    } else {
        (trigger.to_string(), MatchType::Keyword)
    }
}

/// Extract media file ID and type from a message.
fn extract_media(msg: &Message) -> (Option<String>, Option<String>) {
    if let Some(photo) = msg.photo() {
        let largest = photo.iter().max_by_key(|p| p.width * p.height);
        (largest.map(|p| p.file.id.clone()), Some("photo".to_string()))
    } else if let Some(video) = msg.video() {
        (Some(video.file.id.clone()), Some("video".to_string()))
    } else if let Some(animation) = msg.animation() {
        (Some(animation.file.id.clone()), Some("animation".to_string()))
    } else if let Some(document) = msg.document() {
        (Some(document.file.id.clone()), Some("document".to_string()))
    } else if let Some(sticker) = msg.sticker() {
        (Some(sticker.file.id.clone()), Some("sticker".to_string()))
    } else {
        (None, None)
    }
}

//! Filter command handlers.
//!
//! Commands for managing auto-reply filters in groups.

use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters, UserId};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::{Filter, GroupSettingsRepo, MatchType};
use crate::utils::{html_escape, parse_content};

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
    if !state.permissions.can_change_info(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args = text.split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim())
        .unwrap_or("");

    if args.is_empty() {
        // Show help
        bot.send_message(
            chat_id,
            "<b>📖 Cara menggunakan filter:</b>\n\n\
            <code>/filter trigger reply</code>\n\
            <code>/filter \"multi word\" reply</code>\n\n\
            <b>Prefix khusus:</b>\n\
            • <code>exact:trigger</code> - Hanya cocok jika pesan persis sama\n\
            • <code>prefix:trigger</code> - Hanya cocok jika pesan dimulai dengan trigger\n\n\
            <b>Tags:</b>\n\
            • <code>{admin}</code> - Hanya admin yang trigger\n\
            • <code>{user}</code> - Hanya non-admin yang trigger\n\
            • <code>{protect}</code> - Tidak bisa diforward\n\n\
            <b>Random reply:</b>\n\
            Gunakan <code>%%%</code> untuk memisahkan reply acak",
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Parse trigger and reply
    let (trigger, reply) = parse_filter_args(args);

    if trigger.is_empty() {
        bot.send_message(chat_id, "❌ Trigger tidak boleh kosong.")
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
        bot.send_message(chat_id, "❌ Reply tidak boleh kosong. Berikan teks reply atau reply ke media.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Determine match type from trigger prefix
    let (clean_trigger, match_type) = parse_trigger_type(&trigger);

    // Parse content for tags and buttons
    let parsed = parse_content(&final_reply);

    // Create filter
    let mut filter = Filter::new(clean_trigger.clone(), parsed.text.clone());
    filter.match_type = match_type;
    filter.buttons = parsed.buttons;
    filter.media_file_id = media_file_id;
    filter.media_type = media_type;
    filter.admin_only = parsed.tags.admin_only;
    filter.user_only = parsed.tags.user_only;
    filter.protect = parsed.tags.protect;
    filter.replytag = parsed.tags.replytag;

    // Save filter
    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;
    settings.filters.add_filter(filter);
    repo.save(&settings).await?;

    info!("Added filter '{}' in chat {}", clean_trigger, chat_id);

    bot.send_message(
        chat_id,
        format!("✅ Filter <code>{}</code> berhasil ditambahkan!", html_escape(&clean_trigger)),
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

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let settings = repo.get_or_create(chat_id.0).await?;

    if settings.filters.filters.is_empty() {
        bot.send_message(chat_id, "📭 Tidak ada filter di grup ini.\n\nGunakan <code>/filter trigger reply</code> untuk menambahkan.")
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let mut text = format!("<b>📋 Filter di grup ini ({}):</b>\n\n", settings.filters.filters.len());
    for filter in &settings.filters.filters {
        let prefix = match filter.match_type {
            MatchType::Exact => "exact:",
            MatchType::Prefix => "prefix:",
            MatchType::Keyword => "",
        };
        text.push_str(&format!("• <code>{}{}</code>\n", prefix, html_escape(&filter.trigger)));
    }
    text.push_str("\nGunakan <code>/stop trigger</code> untuk menghapus filter.");

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
    if !state.permissions.can_change_info(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let trigger = text.split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim())
        .unwrap_or("");

    if trigger.is_empty() {
        bot.send_message(chat_id, "📖 Gunakan: <code>/stop trigger</code>")
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;

    if settings.filters.remove_filter(trigger) {
        repo.save(&settings).await?;
        info!("Removed filter '{}' from chat {}", trigger, chat_id);

        bot.send_message(
            chat_id,
            format!("✅ Filter <code>{}</code> berhasil dihapus!", html_escape(trigger)),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    } else {
        bot.send_message(
            chat_id,
            format!("❌ Filter <code>{}</code> tidak ditemukan.", html_escape(trigger)),
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

    // Check permission: must be owner or can_change_info
    if !state.permissions.is_owner(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Hanya owner grup yang bisa menghapus semua filter.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;

    let count = settings.filters.clear_all();
    repo.save(&settings).await?;

    info!("Cleared all {} filters from chat {}", count, chat_id);

    bot.send_message(
        chat_id,
        format!("✅ {} filter berhasil dihapus!", count),
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

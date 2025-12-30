//! Bye (Goodbye) command handlers.
//!
//! Commands for configuring goodbye messages in groups.

use teloxide::prelude::*;
use teloxide::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, InputFile, ParseMode, ReplyParameters,
};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::{ByeConfig, GroupSettingsRepo, InlineButton};

/// Handle /bye command - show or toggle goodbye.
pub async fn bye_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(user) => user.id,
        None => return Ok(()),
    };

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, "⚠️ Perintah ini hanya untuk grup.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let settings = repo.get_or_create(chat_id.0).await?;

    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    if args.is_empty() {
        // Show current bye settings
        let status = format_bye_status(&settings.bye);
        bot.send_message(chat_id, status)
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    match args[0].to_lowercase().as_str() {
        "on" | "enable" => {
            let mut new_settings = settings.clone();
            new_settings.bye.enabled = true;
            repo.save(&new_settings).await?;
            bot.send_message(chat_id, "✅ Goodbye message diaktifkan!")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
        "off" | "disable" => {
            let mut new_settings = settings.clone();
            new_settings.bye.enabled = false;
            repo.save(&new_settings).await?;
            bot.send_message(chat_id, "❌ Goodbye message dinonaktifkan!")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
        "preview" => {
            // Show preview of goodbye message
            send_bye_preview(&bot, chat_id, &settings.bye, &msg).await?;
        }
        _ => {
            bot.send_message(
                chat_id,
                "<b>📖 Penggunaan Goodbye</b>\n\n\
                <code>/bye</code> - Lihat status\n\
                <code>/bye on</code> - Aktifkan\n\
                <code>/bye off</code> - Nonaktifkan\n\
                <code>/bye preview</code> - Preview pesan\n\
                <code>/setbye</code> - Atur pesan (reply ke pesan/media)\n\
                <code>/setbyebuttons</code> - Atur tombol\n\
                <code>/resetbye</code> - Reset ke default",
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        }
    }

    Ok(())
}

/// Handle /setbye command - set goodbye message by replying to a message.
pub async fn setbye_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(user) => user.id,
        None => return Ok(()),
    };

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;

    // Check if replying to a message
    let replied = msg.reply_to_message();
    let text_content = msg.text().unwrap_or("");
    let args_text = text_content
        .split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim())
        .filter(|s| !s.is_empty());

    if let Some(reply) = replied {
        // Get message content from reply
        let (message_text, media_file_id, media_type) = extract_message_content(reply);

        if let Some(text) = message_text.or_else(|| args_text.map(String::from)) {
            settings.bye.message = Some(text);
        }

        if let Some(file_id) = media_file_id {
            settings.bye.media_file_id = Some(file_id);
            settings.bye.media_type = media_type;
        }

        repo.save(&settings).await?;
        bot.send_message(chat_id, "✅ Goodbye message berhasil diatur!")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        info!("Goodbye message set in chat {}", chat_id);
    } else if let Some(text) = args_text {
        // Direct text after command
        settings.bye.message = Some(text.to_string());
        settings.bye.media_file_id = None;
        settings.bye.media_type = None;
        repo.save(&settings).await?;
        bot.send_message(chat_id, "✅ Goodbye message berhasil diatur!")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    } else {
        bot.send_message(
            chat_id,
            "<b>📖 Cara mengatur goodbye:</b>\n\n\
            1. Reply ke pesan/media dengan <code>/setbye</code>\n\
            2. Atau: <code>/setbye Selamat tinggal!</code>\n\n\
            <b>Format yang didukung:</b>\n\
            <code>{name}</code> - Nama user\n\
            <code>{username}</code> - Username\n\
            <code>{mention}</code> - Mention user\n\
            <code>{id}</code> - User ID\n\
            <code>{group}</code> - Nama grup",
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    }

    Ok(())
}

/// Handle /setbyebuttons command - set inline buttons.
pub async fn setbyebuttons_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(user) => user.id,
        None => return Ok(()),
    };

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args = text
        .split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim())
        .unwrap_or("");

    if args.is_empty() || args == "clear" {
        let repo = GroupSettingsRepo::new(&state.db, &state.cache);
        let mut settings = repo.get_or_create(chat_id.0).await?;
        settings.bye.buttons.clear();
        repo.save(&settings).await?;

        if args == "clear" {
            bot.send_message(chat_id, "✅ Tombol goodbye dihapus!")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        } else {
            bot.send_message(
                chat_id,
                "<b>📖 Cara mengatur tombol:</b>\n\n\
                <code>/setbyebuttons {button:Teks|url}</code>\n\n\
                Gunakan <code>:same</code> untuk tombol di baris sama:\n\
                <code>{button:Teks1|url1}:same {button:Teks2|url2}</code>\n\n\
                <code>/setbyebuttons clear</code> - Hapus semua tombol",
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        }
        return Ok(());
    }

    // Parse buttons
    let buttons = parse_buttons(args);

    if buttons.is_empty() {
        bot.send_message(chat_id, "❌ Format tombol tidak valid. Gunakan: {button:Teks|url}")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;
    settings.bye.buttons = buttons;
    repo.save(&settings).await?;

    bot.send_message(chat_id, "✅ Tombol goodbye berhasil diatur!")
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

/// Handle /resetbye command.
pub async fn resetbye_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(user) => user.id,
        None => return Ok(()),
    };

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;
    settings.bye = ByeConfig::default();
    repo.save(&settings).await?;

    bot.send_message(chat_id, "✅ Goodbye message direset ke default!")
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

/// Extract message content (text, media file_id, media type).
fn extract_message_content(msg: &Message) -> (Option<String>, Option<String>, Option<String>) {
    let text = msg.text().or_else(|| msg.caption()).map(String::from);

    let (file_id, media_type) = if let Some(photo) = msg.photo() {
        let largest = photo.iter().max_by_key(|p| p.width * p.height);
        (
            largest.map(|p| p.file.id.clone()),
            Some("photo".to_string()),
        )
    } else if let Some(video) = msg.video() {
        (Some(video.file.id.clone()), Some("video".to_string()))
    } else if let Some(animation) = msg.animation() {
        (
            Some(animation.file.id.clone()),
            Some("animation".to_string()),
        )
    } else if let Some(sticker) = msg.sticker() {
        (Some(sticker.file.id.clone()), Some("sticker".to_string()))
    } else if let Some(document) = msg.document() {
        (Some(document.file.id.clone()), Some("document".to_string()))
    } else {
        (None, None)
    };

    (text, file_id, media_type)
}

/// Parse buttons (same format as welcome)
fn parse_buttons(input: &str) -> Vec<Vec<InlineButton>> {
    let mut rows: Vec<Vec<InlineButton>> = vec![];
    let mut current_row: Vec<InlineButton> = vec![];

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' && i + 7 < chars.len() {
            let prefix: String = chars[i..i + 8].iter().collect();
            if prefix.to_lowercase() == "{button:"
                && let Some((btn, end_idx)) = try_parse_button(&chars, i) {
                    current_row.push(btn);
                    i = end_idx;
                    
                    if i < chars.len() && chars[i] == ':' {
                        i += 1;
                        continue;
                    } else {
                        if !current_row.is_empty() {
                            rows.push(current_row);
                            current_row = vec![];
                        }
                        continue;
                    }
                }
        }
        i += 1;
    }

    if !current_row.is_empty() {
        rows.push(current_row);
    }

    rows
}

fn try_parse_button(chars: &[char], start: usize) -> Option<(InlineButton, usize)> {
    if start + 8 >= chars.len() {
        return None;
    }

    let prefix: String = chars[start..start + 8].iter().collect();
    if prefix.to_lowercase() != "{button:" {
        return None;
    }

    let mut i = start + 8;
    let mut text = String::new();
    while i < chars.len() && chars[i] != '|' && chars[i] != '}' {
        text.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '|' {
        return None;
    }
    i += 1;

    let mut url = String::new();
    while i < chars.len() && chars[i] != '}' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '}' {
        return None;
    }
    i += 1;

    let text = text.trim().to_string();
    let url = url.trim().to_string();
    if text.is_empty() || url.is_empty() {
        return None;
    }

    Some((InlineButton { text, url }, i))
}

fn format_bye_status(config: &ByeConfig) -> String {
    let status = if config.enabled { "✅ Aktif" } else { "❌ Nonaktif" };
    let message = config
        .message
        .as_deref()
        .unwrap_or("<i>Tidak ada</i>");
    let media = if config.media_file_id.is_some() {
        format!("✅ {} terlampir", config.media_type.as_deref().unwrap_or("Media"))
    } else {
        "❌ Tidak ada".to_string()
    };
    let buttons = if config.buttons.is_empty() {
        "❌ Tidak ada".to_string()
    } else {
        let count: usize = config.buttons.iter().map(|r| r.len()).sum();
        format!("✅ {} tombol", count)
    };

    format!(
        "<b>👋 Pengaturan Goodbye</b>\n\n\
        <b>Status:</b> {}\n\
        <b>Media:</b> {}\n\
        <b>Tombol:</b> {}\n\n\
        <b>Pesan:</b>\n{}",
        status, media, buttons, message
    )
}

async fn send_bye_preview(
    bot: &ThrottledBot,
    chat_id: ChatId,
    config: &ByeConfig,
    msg: &Message,
) -> anyhow::Result<()> {
    let user = msg.from.as_ref().unwrap();
    let formatted = format_bye_text(
        config.message.as_deref().unwrap_or("Selamat tinggal!"),
        user,
        msg.chat.title().unwrap_or("Grup"),
    );

    let keyboard = build_bye_keyboard(&config.buttons);

    if let Some(ref file_id) = config.media_file_id {
        match config.media_type.as_deref() {
            Some("photo") => {
                bot.send_photo(chat_id, InputFile::file_id(file_id))
                    .caption(formatted)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
            Some("video") => {
                bot.send_video(chat_id, InputFile::file_id(file_id))
                    .caption(formatted)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
            Some("animation") => {
                bot.send_animation(chat_id, InputFile::file_id(file_id))
                    .caption(formatted)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
            _ => {
                bot.send_message(chat_id, formatted)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
        }
    } else {
        bot.send_message(chat_id, formatted)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    }

    Ok(())
}

/// Format goodbye text with placeholders.
pub fn format_bye_text(template: &str, user: &teloxide::types::User, group: &str) -> String {
    let name = user.first_name.clone();
    let username = user
        .username
        .as_ref()
        .map(|u| format!("@{}", u))
        .unwrap_or_else(|| name.clone());
    let mention = format!(
        "<a href=\"tg://user?id={}\">{}</a>",
        user.id,
        html_escape(&name)
    );

    template
        .replace("{name}", &html_escape(&name))
        .replace("{username}", &html_escape(&username))
        .replace("{mention}", &mention)
        .replace("{id}", &user.id.to_string())
        .replace("{group}", &html_escape(group))
}

pub fn build_bye_keyboard(buttons: &[Vec<InlineButton>]) -> InlineKeyboardMarkup {
    let keyboard: Vec<Vec<InlineKeyboardButton>> = buttons
        .iter()
        .map(|row| {
            row.iter()
                .filter_map(|btn| {
                    btn.url.parse().ok().map(|url| {
                        InlineKeyboardButton::url(&btn.text, url)
                    })
                })
                .collect()
        })
        .collect();

    InlineKeyboardMarkup::new(keyboard)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

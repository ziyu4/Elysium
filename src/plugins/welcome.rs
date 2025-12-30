//! Welcome command handlers.
//!
//! Commands for configuring welcome messages in groups.

use teloxide::prelude::*;
use teloxide::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, InputFile, ParseMode, ReplyParameters,
};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::{GroupSettingsRepo, InlineButton, WelcomeConfig};

/// Handle /welcome command - show or toggle welcome.
pub async fn welcome_command(
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
        // Show current welcome settings
        let status = format_welcome_status(&settings.welcome);
        bot.send_message(chat_id, status)
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    match args[0].to_lowercase().as_str() {
        "on" | "enable" => {
            let mut new_settings = settings.clone();
            new_settings.welcome.enabled = true;
            repo.save(&new_settings).await?;
            bot.send_message(chat_id, "✅ Welcome message diaktifkan!")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
        "off" | "disable" => {
            let mut new_settings = settings.clone();
            new_settings.welcome.enabled = false;
            repo.save(&new_settings).await?;
            bot.send_message(chat_id, "❌ Welcome message dinonaktifkan!")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
        "preview" => {
            // Show preview of welcome message
            send_welcome_preview(&bot, chat_id, &settings.welcome, &msg).await?;
        }
        _ => {
            bot.send_message(
                chat_id,
                "<b>📖 Penggunaan Welcome</b>\n\n\
                <code>/welcome</code> - Lihat status\n\
                <code>/welcome on</code> - Aktifkan\n\
                <code>/welcome off</code> - Nonaktifkan\n\
                <code>/welcome preview</code> - Preview pesan\n\
                <code>/setwelcome</code> - Atur pesan (reply ke pesan/media)\n\
                <code>/setwelcomebuttons</code> - Atur tombol\n\
                <code>/resetwelcome</code> - Reset ke default",
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        }
    }

    Ok(())
}

/// Handle /setwelcome command - set welcome message by replying to a message.
pub async fn setwelcome_command(
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
            settings.welcome.message = Some(text);
        }

        if let Some(file_id) = media_file_id {
            settings.welcome.media_file_id = Some(file_id);
            settings.welcome.media_type = media_type;
        }

        repo.save(&settings).await?;
        bot.send_message(chat_id, "✅ Welcome message berhasil diatur!")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        info!("Welcome message set in chat {}", chat_id);
    } else if let Some(text) = args_text {
        // Direct text after command
        settings.welcome.message = Some(text.to_string());
        settings.welcome.media_file_id = None;
        settings.welcome.media_type = None;
        repo.save(&settings).await?;
        bot.send_message(chat_id, "✅ Welcome message berhasil diatur!")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    } else {
        bot.send_message(
            chat_id,
            "<b>📖 Cara mengatur welcome:</b>\n\n\
            1. Reply ke pesan/media dengan <code>/setwelcome</code>\n\
            2. Atau: <code>/setwelcome Selamat datang!</code>\n\n\
            <b>Format yang didukung:</b>\n\
            <code>{name}</code> - Nama user\n\
            <code>{username}</code> - Username\n\
            <code>{mention}</code> - Mention user\n\
            <code>{id}</code> - User ID\n\
            <code>{group}</code> - Nama grup\n\
            <code>{count}</code> - Jumlah member",
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    }

    Ok(())
}

/// Handle /setwelcomebuttons command - set inline buttons.
pub async fn setwelcomebuttons_command(
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
        // Clear buttons
        let repo = GroupSettingsRepo::new(&state.db, &state.cache);
        let mut settings = repo.get_or_create(chat_id.0).await?;
        settings.welcome.buttons.clear();
        repo.save(&settings).await?;

        if args == "clear" {
            bot.send_message(chat_id, "✅ Tombol welcome dihapus!")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        } else {
            bot.send_message(
                chat_id,
                "<b>📖 Cara mengatur tombol:</b>\n\n\
                <code>/setwelcomebuttons {button:Teks|url}</code>\n\n\
                Gunakan <code>:same</code> untuk tombol di baris sama:\n\
                <code>{button:Teks1|url1}:same {button:Teks2|url2}</code>\n\n\
                <b>Contoh:</b>\n\
                <code>/setwelcomebuttons {button:📜 Rules|t.me/bot?start=rules}</code>\n\
                <code>/setwelcomebuttons {button:📜 Rules|url}:same {button:📢 Channel|url}</code>\n\n\
                <code>/setwelcomebuttons clear</code> - Hapus semua tombol",
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        }
        return Ok(());
    }

    // Parse buttons: [Text](url) | [Text](url)
    let buttons = parse_buttons(args);

    if buttons.is_empty() {
        bot.send_message(chat_id, "❌ Format tombol tidak valid. Gunakan: {button:Teks|url}")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;
    settings.welcome.buttons = buttons;
    repo.save(&settings).await?;

    bot.send_message(chat_id, "✅ Tombol welcome berhasil diatur!")
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

/// Handle /resetwelcome command.
pub async fn resetwelcome_command(
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
    settings.welcome = WelcomeConfig::default();
    repo.save(&settings).await?;

    bot.send_message(chat_id, "✅ Welcome message direset ke default!")
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

/// Extract message content (text, media file_id, media type).
fn extract_message_content(msg: &Message) -> (Option<String>, Option<String>, Option<String>) {
    let text = msg.text().or_else(|| msg.caption()).map(String::from);

    let (file_id, media_type) = if let Some(photo) = msg.photo() {
        // Get largest photo
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

/// Parse button format: {button:Text|url}
/// - Colon `:` between buttons = same row
/// - Space/newline = different rows
fn parse_buttons(input: &str) -> Vec<Vec<InlineButton>> {
    let mut rows: Vec<Vec<InlineButton>> = vec![];
    let mut current_row: Vec<InlineButton> = vec![];

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for {button: pattern
        if chars[i] == '{' && i + 7 < chars.len() {
            let prefix: String = chars[i..i + 8].iter().collect();
            if prefix.to_lowercase() == "{button:" {
                // Try to parse button
                if let Some((btn, end_idx)) = try_parse_welcome_button(&chars, i) {
                    current_row.push(btn);
                    i = end_idx;
                    
                    // Check what comes after: colon means same row, else new row
                    if i < chars.len() && chars[i] == ':' {
                        // Skip colon, continue to next button (same row)
                        i += 1;
                        continue;
                    } else {
                        // Space, newline, or other - push row and start new one
                        if !current_row.is_empty() {
                            rows.push(current_row);
                            current_row = vec![];
                        }
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    // Push last row
    if !current_row.is_empty() {
        rows.push(current_row);
    }

    rows
}

/// Try to parse a button: {button:Text|url}
fn try_parse_welcome_button(chars: &[char], start: usize) -> Option<(InlineButton, usize)> {
    if start + 8 >= chars.len() {
        return None;
    }

    let prefix: String = chars[start..start + 8].iter().collect();
    if prefix.to_lowercase() != "{button:" {
        return None;
    }

    let mut i = start + 8;

    // Find the | separator
    let mut text = String::new();
    while i < chars.len() && chars[i] != '|' && chars[i] != '}' {
        text.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '|' {
        return None;
    }
    i += 1; // skip |

    // Find closing }
    let mut url = String::new();
    while i < chars.len() && chars[i] != '}' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '}' {
        return None;
    }
    i += 1; // skip }

    // Validate
    let text = text.trim().to_string();
    let url = url.trim().to_string();
    if text.is_empty() || url.is_empty() {
        return None;
    }

    Some((InlineButton { text, url }, i))
}

/// Format welcome status for display.
fn format_welcome_status(config: &WelcomeConfig) -> String {
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
        "<b>🎉 Pengaturan Welcome</b>\n\n\
        <b>Status:</b> {}\n\
        <b>Media:</b> {}\n\
        <b>Tombol:</b> {}\n\n\
        <b>Pesan:</b>\n{}",
        status, media, buttons, message
    )
}

/// Send welcome preview.
async fn send_welcome_preview(
    bot: &ThrottledBot,
    chat_id: ChatId,
    config: &WelcomeConfig,
    msg: &Message,
) -> anyhow::Result<()> {
    let user = msg.from.as_ref().unwrap();
    let formatted = format_welcome_text(
        config.message.as_deref().unwrap_or("Selamat datang!"),
        user,
        &msg.chat.title().unwrap_or("Grup"),
        0, // member count placeholder
    );

    let keyboard = build_welcome_keyboard(&config.buttons);

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

/// Format welcome text with placeholders.
pub fn format_welcome_text(template: &str, user: &teloxide::types::User, group: &str, count: u64) -> String {
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
        .replace("{count}", &count.to_string())
}

/// Build inline keyboard from buttons config.
pub fn build_welcome_keyboard(buttons: &[Vec<InlineButton>]) -> InlineKeyboardMarkup {
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

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

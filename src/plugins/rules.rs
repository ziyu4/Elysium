//! Rules command handlers.
//!
//! Commands for setting and viewing group rules.

use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, ReplyParameters};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::GroupSettingsRepo;

/// Handle /rules command - show group rules.
pub async fn rules_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, "⚠️ Perintah ini hanya untuk grup.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let settings = repo.get_or_create(chat_id.0).await?;

    let rules_text = match &settings.rules.text {
        Some(text) if !text.trim().is_empty() => text,
        _ => {
            bot.send_message(chat_id, "📜 Belum ada peraturan yang ditetapkan untuk grup ini.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    if settings.rules.show_in_pm {
        // Show button to view in PM using state.bot_username
        let deep_link = format!(
            "https://t.me/{}?start=rules_{}",
            state.bot_username, chat_id.0
        );

        let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::url(
            &settings.rules.button_text,
            deep_link.parse().unwrap(),
        )]]);

        bot.send_message(chat_id, "📜 Klik tombol di bawah untuk membaca peraturan grup.")
            .reply_markup(keyboard)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    } else {
        // Show rules directly in group
        let title = msg.chat.title().unwrap_or("Grup");
        let formatted = format!(
            "<b>📜 Peraturan {}</b>\n\n{}",
            html_escape(title),
            rules_text
        );

        bot.send_message(chat_id, formatted)
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    }

    Ok(())
}

/// Handle /setrules command - set group rules.
pub async fn setrules_command(
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

    // Get rules text from reply or command args
    let rules_text = get_rules_text(&msg);

    if rules_text.is_none() {
        bot.send_message(
            chat_id,
            "<b>📖 Cara mengatur peraturan:</b>\n\n\
            1. Reply ke pesan dengan <code>/setrules</code>\n\
            2. Atau: <code>/setrules Peraturan grup ini adalah...</code>\n\n\
            Teks mendukung format HTML dan multi-baris.",
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    let rules_text = rules_text.unwrap();

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;
    settings.rules.text = Some(rules_text);
    repo.save(&settings).await?;

    bot.send_message(chat_id, "✅ Peraturan grup berhasil diatur!\nGunakan /rules untuk melihat.")
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    info!("Rules set in chat {}", chat_id);
    Ok(())
}

/// Handle /clearrules command.
pub async fn clearrules_command(
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
    settings.rules.text = None;
    repo.save(&settings).await?;

    bot.send_message(chat_id, "✅ Peraturan grup telah dihapus.")
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

/// Handle /setrulesprivate command - toggle rules display mode.
pub async fn setrulesprivate_command(
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
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;

    if args.is_empty() {
        // Toggle
        settings.rules.show_in_pm = !settings.rules.show_in_pm;
    } else {
        match args[0].to_lowercase().as_str() {
            "on" | "yes" | "true" | "pm" => settings.rules.show_in_pm = true,
            "off" | "no" | "false" | "group" => settings.rules.show_in_pm = false,
            _ => {
                bot.send_message(
                    chat_id,
                    "📖 Gunakan: /setrulesprivate on/off\n\
                    on = Tampilkan di PM\n\
                    off = Tampilkan di grup",
                )
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
                return Ok(());
            }
        }
    }

    repo.save(&settings).await?;

    let mode = if settings.rules.show_in_pm {
        "di PM (pesan pribadi)"
    } else {
        "langsung di grup"
    };

    bot.send_message(chat_id, format!("✅ Peraturan akan ditampilkan {}.", mode))
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

/// Handle deep link for rules: /start rules_CHATID
pub async fn handle_rules_deeplink(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    chat_id_str: &str,
) -> anyhow::Result<()> {
    let private_chat_id = msg.chat.id;

    // Parse chat ID from deep link
    let group_chat_id: i64 = match chat_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            bot.send_message(private_chat_id, "❌ Link tidak valid.")
                .await?;
            return Ok(());
        }
    };

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let settings = match repo.get(group_chat_id).await? {
        Some(s) => s,
        None => {
            bot.send_message(private_chat_id, "❌ Grup tidak ditemukan.")
                .await?;
            return Ok(());
        }
    };

    let rules_text = match &settings.rules.text {
        Some(text) if !text.trim().is_empty() => text,
        _ => {
            bot.send_message(private_chat_id, "📜 Belum ada peraturan untuk grup ini.")
                .await?;
            return Ok(());
        }
    };

    let group_name = settings.title.as_deref().unwrap_or("Grup");
    let formatted = format!(
        "<b>📜 Peraturan {}</b>\n\n{}",
        html_escape(group_name),
        rules_text
    );

    bot.send_message(private_chat_id, formatted)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

/// Get rules text from message (reply or args).
fn get_rules_text(msg: &Message) -> Option<String> {
    // Check if replying to a message
    if let Some(reply) = msg.reply_to_message() {
        if let Some(text) = reply.text().or_else(|| reply.caption()) {
            return Some(text.to_string());
        }
    }

    // Check command args
    let text = msg.text()?;
    let args = text
        .split_once(char::is_whitespace)
        .map(|(_, rest)| rest.trim())
        .filter(|s| !s.is_empty());

    args.map(String::from)
}

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

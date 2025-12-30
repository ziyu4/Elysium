//! /start command plugin.
//!
//! Handles the /start command and sends a welcome message.

use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

use crate::bot::dispatcher::{AppState, ThrottledBot};

/// Handle the /start command.
pub async fn start_handler(
    bot: ThrottledBot,
    msg: Message,
    _state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    let welcome_text = r#"*Halo\!* 👋

Saya adalah *Elysium*, bot manajemen grup Telegram\.

*Fitur:*
• Antiflood
• Welcome/Goodbye
• AFK
• Notes
• Filters
• Admin commands

Gunakan /help untuk melihat daftar perintah\."#;

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::url("👨‍💻 Developer", "https://github.com/ziyu4".parse().unwrap()),
        ],
    ]);

    bot.send_message(chat_id, welcome_text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

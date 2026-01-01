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
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    let welcome_text = crate::i18n::get_text(&locale, "start.welcome");

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::url(crate::i18n::get_text(&locale, "start.btn_dev"), "https://github.com/ziyu4".parse().unwrap()),
        ],
    ]);

    bot.send_message(chat_id, welcome_text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

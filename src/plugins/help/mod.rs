//! Help command module.
//!
//! Handles /help command and callback queries for the interactive help system.

use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

use crate::bot::dispatcher::{AppState, ThrottledBot};

mod main_help;
mod notes;
mod afk;
mod admin;
mod filters;
mod welcome;
mod bye;
mod warns;
mod antiflood;
mod approval;
mod purge;
mod rules;

/// Handle /help command.
pub async fn help_handler(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    // In groups, redirect to PM
    if msg.chat.is_group() || msg.chat.is_supergroup() {
        let pm_url = format!("https://t.me/{}?start=help", state.bot_username);
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::url("📚 Bantuan", pm_url.parse().unwrap())],
        ]);
        bot.send_message(chat_id, "Hubungi saya di PM.")
            .reply_markup(keyboard)
            .await?;
        return Ok(());
    }

    // In PM, show help directly
    send_help_menu(&bot, chat_id).await
}

/// Send the main help menu.
pub async fn send_help_menu(bot: &ThrottledBot, chat_id: ChatId) -> anyhow::Result<()> {
    let text = main_help::get_text();
    let keyboard = main_help::get_keyboard();

    bot.send_message(chat_id, text)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

/// Handle help callback queries (help:*).
pub async fn callback_handler(
    bot: ThrottledBot,
    q: CallbackQuery,
    _state: AppState,
) -> anyhow::Result<()> {
    let data = match q.data {
        Some(d) => d,
        None => return Ok(()),
    };

    if !data.starts_with("help:") {
        return Ok(());
    }

    let part = data.strip_prefix("help:").unwrap_or("");
    let (text, keyboard) = match part {
        "main" | "back" => (main_help::get_text().to_string(), main_help::get_keyboard()),
        "notes" => (notes::get_text(), notes::get_keyboard()),
        "afk" => (afk::get_text(), afk::get_keyboard()),
        "admin" => (admin::get_text(), admin::get_keyboard()),
        "filters" => (filters::get_text(), filters::get_keyboard()),
        "welcome" => (welcome::get_text(), welcome::get_keyboard()),
        "bye" => (bye::get_text(), bye::get_keyboard()),
        "warns" => (warns::get_text(), warns::get_keyboard()),
        "antiflood" => (antiflood::get_text(), antiflood::get_keyboard()),
        "approval" => (approval::get_text(), approval::get_keyboard()),
        "purge" => (purge::get_text(), purge::get_keyboard()),
        "rules" => (rules::get_text(), rules::get_keyboard()),
        _ => return Ok(()),
    };

    if let Some(msg) = q.message {
        bot.edit_message_text(msg.chat().id, msg.id(), text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}

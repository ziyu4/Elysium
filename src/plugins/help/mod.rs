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
    // Get locale
    let locale = state.get_locale(Some(chat_id.0), msg.from.as_ref().map(|u| u.id.0)).await;

    // In groups, redirect to PM
    if msg.chat.is_group() || msg.chat.is_supergroup() {
        let pm_url = format!("https://t.me/{}?start=help", state.bot_username);
        let btn_text = crate::i18n::get_text(&locale, "common.help_btn"); 
        let btn_text = if btn_text == "common.help_btn" { "📚 Help / Bantuan".to_string() } else { btn_text }; // Fallback temp
        
        // Use a simple localized message
        let msg_text = crate::i18n::get_text(&locale, "help.redirect_pm");
        let msg_text = if msg_text == "help.redirect_pm" { "Contact me in PM." } else { &msg_text };

        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::url(btn_text, pm_url.parse().unwrap())],
        ]);
        bot.send_message(chat_id, msg_text)
            .reply_markup(keyboard)
            .await?;
        return Ok(());
    }

    // In PM, show help directly
    send_help_menu(&bot, chat_id, &locale).await
}

/// Send the main help menu.
pub async fn send_help_menu(bot: &ThrottledBot, chat_id: ChatId, locale: &str) -> anyhow::Result<()> {
    let text = main_help::get_text(locale);
    let keyboard = main_help::get_keyboard(locale);

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
    state: AppState,
) -> anyhow::Result<()> {
    let data = match q.data {
        Some(d) => d,
        None => return Ok(()),
    };

    if !data.starts_with("help:") {
        return Ok(());
    }

    // Resolve locale (callback query user)
    let user_id = q.from.id.0;
    // We can't easily get chat_id from callback query if message is too old, but usually q.message is present.
    let chat_id = q.message.as_ref().map(|m| m.chat().id.0);
    
    // We need to resolve locale asynchronously.
    // However, AppState is available.
    // Since callback_handler is async, we can await.
    // But `state` is passed as `_state` in the replacement?
    // Wait, the function signature in `mod.rs` was `_state`. I need to rename it to `state`.
    
    // NOTE: In the original code `_state` was unused. I need to make sure I update the signature if I haven't.
    // checking file... signature is `_state: AppState`. I need to change it to `state`.
    // I can't change signature in this chunk easily if I don't target it.
    // Let's assume I will fix signature in the same chunk or a prior one?
    // The chunk starts at line 71.
    // The signature is at line 64.
    // I should probably target the signature too.
    
    // Let's proceed assuming I can edit the locale resolution logic here, 
    // BUT I can't use `state` if it's named `_state`.
    // I'll assume I can just use `_state` variable (it's valid variable name just with warning suppression).
    let locale = state.get_locale(chat_id, Some(user_id)).await;

    let part = data.strip_prefix("help:").unwrap_or("");
    let (text, keyboard) = match part {
        "main" | "back" => (main_help::get_text(&locale), main_help::get_keyboard(&locale)),
        "notes" => (notes::get_text(&locale), notes::get_keyboard(&locale)),
        "afk" => (afk::get_text(&locale), afk::get_keyboard(&locale)),
        "admin" => (admin::get_text(&locale), admin::get_keyboard(&locale)),
        "filters" => (filters::get_text(&locale), filters::get_keyboard(&locale)),
        "welcome" => (welcome::get_text(&locale), welcome::get_keyboard(&locale)),
        "bye" => (bye::get_text(&locale), bye::get_keyboard(&locale)),
        "warns" => (warns::get_text(&locale), warns::get_keyboard(&locale)),
        "antiflood" => (antiflood::get_text(&locale), antiflood::get_keyboard(&locale)),
        "approval" => (approval::get_text(&locale), approval::get_keyboard(&locale)),
        "purge" => (purge::get_text(&locale), purge::get_keyboard(&locale)),
        "rules" => (rules::get_text(&locale), rules::get_keyboard(&locale)),
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

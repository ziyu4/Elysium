//! Rules command handlers.
//!
//! Commands for setting and viewing group rules.
//! Refactored to use decentralized RulesRepository.

use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, ReplyParameters};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::i18n::get_text;

/// Handle /rules command - show group rules.
pub async fn rules_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        let locale = state.get_locale(Some(chat_id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;
        bot.send_message(chat_id, get_text(&locale, "rules.error_group_only"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let locale = state.get_locale(Some(chat_id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;

    let settings = state.rules.get_or_create(chat_id.0).await?;

    let rules_text = match &settings.text {
        Some(text) if !text.trim().is_empty() => text,
        _ => {
            bot.send_message(chat_id, get_text(&locale, "rules.none_setup"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    if settings.show_in_pm {
        // Show button to view in PM using state.bot_username
        let deep_link = format!(
            "https://t.me/{}?start=rules_{}",
            state.bot_username, chat_id.0
        );

        let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::url(
            &settings.button_text,
            deep_link.parse().unwrap(),
        )]]);

        bot.send_message(chat_id, get_text(&locale, "rules.pm_click_text"))
            .reply_markup(keyboard)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    } else {
        // Show rules directly in group
        let title = msg.chat.title().unwrap_or("Grup");
        let formatted = get_text(&locale, "rules.title_format")
            .replace("{title}", &html_escape(title))
            .replace("{text}", rules_text);

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
        let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;
        bot.send_message(chat_id, get_text(&locale, "rules.error_group_only"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanChangeInfo"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Get rules text from reply or command args
    let rules_text = get_rules_text(&msg);

    if rules_text.is_none() {
        bot.send_message(
            chat_id,
            get_text(&locale, "rules.set_usage"),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    let rules_text = rules_text.unwrap();

    // Use RulesRepository specific method
    state.rules.set_rules(chat_id.0, Some(rules_text)).await?;

    bot.send_message(chat_id, get_text(&locale, "rules.set_success"))
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

    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanChangeInfo"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Use RulesRepository specific method
    state.rules.clear_rules(chat_id.0).await?;

    bot.send_message(chat_id, get_text(&locale, "rules.cleared"))
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

    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
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
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    let mut settings = state.rules.get_or_create(chat_id.0).await?;

    if args.is_empty() {
        // Toggle
        settings.show_in_pm = !settings.show_in_pm;
    } else {
        match args[0].to_lowercase().as_str() {
            "on" | "yes" | "true" | "pm" => settings.show_in_pm = true,
            "off" | "no" | "false" | "group" => settings.show_in_pm = false,
            _ => {
                bot.send_message(
                    chat_id,
                    get_text(&locale, "rules.private_usage"),
                )
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
                return Ok(());
            }
        }
    }

    state.rules.save(&settings).await?;

    let mode = if settings.show_in_pm {
        get_text(&locale, "rules.private_mode")
    } else {
        get_text(&locale, "rules.group_mode")
    };

    bot.send_message(chat_id, get_text(&locale, "rules.private_set").replace("{mode}", &mode))
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
            let locale = state.get_locale(Some(private_chat_id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;
            bot.send_message(private_chat_id, get_text(&locale, "rules.error_link"))
                .await?;
            return Ok(());
        }
    };

    let locale = state.get_locale(Some(private_chat_id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;

    let settings = match state.rules.get(group_chat_id).await? {
        Some(s) => s,
        None => {
            bot.send_message(private_chat_id, get_text(&locale, "rules.error_group_not_found"))
                .await?;
            return Ok(());
        }
    };

    let rules_text = match &settings.text {
        Some(text) if !text.trim().is_empty() => text,
        _ => {
            bot.send_message(private_chat_id, get_text(&locale, "rules.deeplink_none"))
                .await?;
            return Ok(());
        }
    };

    // This provides robustness against stale titles
    let group_name = match bot.get_chat(ChatId(group_chat_id)).await {
        Ok(chat) => chat.title().map(|t| t.to_string()).unwrap_or("Grup".to_string()),
        Err(_) => "Grup".to_string(),
    };

    let formatted = get_text(&locale, "rules.title_format")
        .replace("{title}", &html_escape(&group_name))
        .replace("{text}", rules_text);

    bot.send_message(private_chat_id, formatted)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

/// Get rules text from message (reply or args).
fn get_rules_text(msg: &Message) -> Option<String> {
    // Check if replying to a message
    if let Some(reply) = msg.reply_to_message()
        && let Some(text) = reply.text().or_else(|| reply.caption()) {
            return Some(text.to_string());
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

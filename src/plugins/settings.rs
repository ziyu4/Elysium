//! Settings plugin.
//!
//! Handles configuration commands like /setlang.

use teloxide::prelude::*;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::models::message_context::GroupInfo;
use crate::i18n::get_text;

pub async fn setlang_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    // Resolve locale for usage/errors
    let locale = state.get_locale(Some(chat_id.0), Some(user_id)).await;

    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().collect();
    
    // /setlang <lang>
    if args.len() < 2 {
        bot.send_message(msg.chat.id, get_text(&locale, "settings.usage"))
            .reply_parameters(teloxide::types::ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }
    
    let lang = args[1].to_lowercase();
    set_lang(bot, msg, state, lang, locale).await
}

async fn set_lang(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    lang: String,
    locale: String, // Current locale for errors
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    
    // Validate lang
    if lang != "en" && lang != "id" {
        bot.send_message(chat_id, get_text(&locale, "settings.invalid_lang"))
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }

    if msg.chat.is_private() {
        // Set User Lang
        state.users.set_lang(user_id, lang.clone()).await?;
        // Use new lang for success message
        bot.send_message(chat_id, get_text(&lang, "settings.success_user")).await?;
    } else {
        // Set Group Lang (Admin Only)
        // Check permission using can_change_info
        if !state.permissions.can_change_info(chat_id, msg.from.as_ref().unwrap().id).await.unwrap_or(false) {
            bot.send_message(
                chat_id,
                get_text(&locale, "common.error_missing_permission")
                    .replace("{permission}", "CanChangeInfo"),
            )
            .await?;
            return Ok(());
        }

        // Need to fetch title for GroupInfo
        let title = msg.chat.title().unwrap_or("Unknown Group").to_string();
        
        let info = GroupInfo {
            id: chat_id.0,
            title,
            lang: Some(lang.clone()),
        };

        state.message_context.update_group_info(chat_id.0, info).await?;
        
        // Use new lang for success message
        bot.send_message(chat_id, get_text(&lang, "settings.success_group"))
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
    }

    Ok(())
}

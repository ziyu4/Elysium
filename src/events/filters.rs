//! Filter event handler.
//!
//! Handles incoming messages and checks for filter triggers.
//! Optimized for decentralized architecture (L1/L2 Caching).

use teloxide::prelude::*;
use teloxide::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, InputFile, MessageId, ParseMode,
    ReplyParameters,
};
use tracing::debug;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::models::DbFilter;
use crate::utils::apply_fillings_new;

/// Public function to check filters - called from unified handler.
pub async fn check_filters(
    bot: &ThrottledBot,
    msg: &Message,
    state: &AppState,
) -> anyhow::Result<()> {
    // Clone values needed for the internal handler
    filter_check_impl(bot, msg, state).await
}

/// Internal filter check implementation.
async fn filter_check_impl(
    bot: &ThrottledBot,
    msg: &Message,
    state: &AppState,
) -> anyhow::Result<()> {
    // Only process in groups
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };

    // Ignore commands
    if text.starts_with('/') {
        return Ok(());
    }

    let chat_id = msg.chat.id;
    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    debug!("Filter handler processing message: '{}' in chat {}", text, chat_id);

    // L1 Cache: Get list of triggers (Strings only)
    let triggers = state.filters.get_triggers(chat_id.0).await?;

    if triggers.is_empty() {
        return Ok(());
    }

    // Find first matching trigger
    // We check against the keys in memory (Fast!)
    let mut matched_trigger = None;
    let text_lower = text.to_lowercase();

    for trigger in triggers {
        // Simple logic first: Checks containment. 
        // Note: Real DBFilter has MatchType (Exact, Prefix, etc).
        // Since L1 only stores the trigger string, we might over-fetch slightly if the 
        // trigger is "cat" and message is "category", but then L2 checks MatchType.
        // For standard "Keyword" matching which is default, this is correct.
        if text_lower.contains(&trigger) {
            matched_trigger = Some(trigger);
            break;
        }
    }

    let trigger = match matched_trigger {
        Some(t) => t,
        None => return Ok(()),
    };
    
    debug!("L1 Filter Match: '{}' in chat {}", trigger, chat_id);

    // L2 Cache: Fetch the actual filter content match
    let filter = match state.filters.get_filter(chat_id.0, &trigger).await? {
        Some(f) => f,
        None => return Ok(()), // Should not happen if L1 and DB are synced
    };

    // Verify MatchType (Double Check)
    if !filter.matches(text) {
        return Ok(());
    }

    // Check user permissions
    let is_admin = state.permissions.is_admin(chat_id, user.id).await.unwrap_or(false);

    if filter.admin_only && !is_admin {
        return Ok(());
    }
    if filter.user_only && is_admin {
        return Ok(());
    }

    debug!("Executing Filter '{}'", filter.trigger);

    // Determine reply target
    let reply_to = if filter.replytag {
        msg.reply_to_message()
            .and_then(|m| m.from.as_ref())
            .map(|_| msg.reply_to_message().unwrap().id)
            .unwrap_or(msg.id)
    } else {
        msg.id
    };

    // Send filter response
    send_filter_response(bot, state, chat_id, user, &filter, reply_to).await?;

    Ok(())
}

/// Send the filter response.
async fn send_filter_response(
    bot: &ThrottledBot,
    _state: &AppState,
    chat_id: ChatId,
    user: &teloxide::types::User,
    filter: &DbFilter,
    reply_to: MessageId,
) -> anyhow::Result<()> {
    let chat_name = "Grup"; 

    // Apply fillings
    let text = apply_fillings_new(&filter.reply, user, chat_name, None);

    // Build keyboard
    let keyboard = if !filter.buttons.is_empty() {
        let rows: Vec<Vec<InlineKeyboardButton>> = filter
            .buttons
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
            .filter(|row: &Vec<_>| !row.is_empty())
            .collect();
        
        if rows.is_empty() {
            None
        } else {
            Some(InlineKeyboardMarkup::new(rows))
        }
    } else {
        None
    };

    // Send based on media type
    match (&filter.media_file_id, &filter.media_type) {
        (Some(file_id), Some(media_type)) => {
            match media_type.as_str() {
                "photo" => {
                    let mut req = bot.send_photo(chat_id, InputFile::file_id(file_id));
                    if !text.is_empty() {
                        req = req.caption(&text).parse_mode(ParseMode::Html);
                    }
                    if let Some(kb) = keyboard {
                        req = req.reply_markup(kb);
                    }
                    if filter.protect {
                        req = req.protect_content(true);
                    }
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                    req.await?;
                }
                "video" => {
                    let mut req = bot.send_video(chat_id, InputFile::file_id(file_id));
                    if !text.is_empty() {
                        req = req.caption(&text).parse_mode(ParseMode::Html);
                    }
                    if let Some(kb) = keyboard {
                        req = req.reply_markup(kb);
                    }
                    if filter.protect {
                        req = req.protect_content(true);
                    }
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                    req.await?;
                }
                "animation" => {
                    let mut req = bot.send_animation(chat_id, InputFile::file_id(file_id));
                    if !text.is_empty() {
                        req = req.caption(&text).parse_mode(ParseMode::Html);
                    }
                    if let Some(kb) = keyboard {
                        req = req.reply_markup(kb);
                    }
                    if filter.protect {
                        req = req.protect_content(true);
                    }
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                    req.await?;
                }
                "document" => {
                    let mut req = bot.send_document(chat_id, InputFile::file_id(file_id));
                    if !text.is_empty() {
                        req = req.caption(&text).parse_mode(ParseMode::Html);
                    }
                    if let Some(kb) = keyboard {
                        req = req.reply_markup(kb);
                    }
                    if filter.protect {
                        req = req.protect_content(true);
                    }
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                    req.await?;
                }
                "sticker" => {
                    let mut req = bot.send_sticker(chat_id, InputFile::file_id(file_id));
                    if filter.protect {
                        req = req.protect_content(true);
                    }
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                    req.await?;
                }
                _ => {}
            }
        }
        _ => {
            // Text-only response
            if !text.is_empty() {
                let mut req = bot.send_message(chat_id, &text)
                    .parse_mode(ParseMode::Html)
                    .reply_parameters(ReplyParameters::new(reply_to));
                if let Some(kb) = keyboard {
                    req = req.reply_markup(kb);
                }
                if filter.protect {
                    req = req.protect_content(true);
                }
                req.await?;
            }
        }
    }

    Ok(())
}

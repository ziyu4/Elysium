//! Welcome event handler.
//!
//! Handles new member joins and sends customizable welcome messages.

use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;
use teloxide::types::{ChatMemberUpdated, InputFile, ParseMode};
use tracing::{debug, info};

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::plugins::welcome::{build_welcome_keyboard, format_welcome_text};
use crate::i18n::get_text;

/// Returns the handler for new member events.
pub fn handler() -> UpdateHandler<anyhow::Error> {
    dptree::filter(is_new_member).endpoint(welcome_handler)
}

/// Check if this is a new member joining.
fn is_new_member(update: ChatMemberUpdated) -> bool {
    let old = &update.old_chat_member;
    let new = &update.new_chat_member;

    // Check if user wasn't a member before and is now
    // Also exclude bots unless explicitly configured
    let is_joining = !old.is_present() && new.is_present();
    let is_not_bot = !new.user.is_bot;

    is_joining && is_not_bot
}

/// Handle new member join event.
async fn welcome_handler(
    bot: ThrottledBot,
    update: ChatMemberUpdated,
    state: AppState,
) -> anyhow::Result<()> {
    let chat = update.chat;
    let user = &update.new_chat_member.user;

    debug!("New member {} joined chat {}", user.id, chat.id);

    // Resolve locale for this chat (using group config first)
    let locale = state.get_locale(Some(chat.id.0), Some(user.id.0)).await;

    // Get welcome settings (lazy loaded, 5min TTL)
    let settings = match state.welcome.get(chat.id.0).await? {
        Some(s) => s,
        None => {
            debug!("No welcome settings for chat {}", chat.id);
            return Ok(());
        }
    };

    // Check if welcome is enabled
    if !settings.enabled {
        debug!("Welcome disabled for chat {}", chat.id);
        return Ok(());
    }

    // Get welcome message text
    let default_msg = get_text(&locale, "welcome.default_message");
    let template = settings
        .message
        .as_deref()
        .unwrap_or(&default_msg);

    let chat_title = chat.title().unwrap_or("Grup");

    // Get member count (optional, may fail)
    let member_count = bot
        .get_chat_member_count(chat.id)
        .await
        .unwrap_or(0) as u64;

    // Format the welcome text with placeholders
    let formatted_text = format_welcome_text(template, user, chat_title, member_count);

    // Build keyboard if buttons are configured
    let keyboard = build_welcome_keyboard(&settings.buttons);

    // Send welcome message (with or without media)
    if let Some(ref file_id) = settings.media_file_id {
        match settings.media_type.as_deref() {
            Some("photo") => {
                bot.send_photo(chat.id, InputFile::file_id(file_id))
                    .caption(formatted_text)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .await?;
            }
            Some("video") => {
                bot.send_video(chat.id, InputFile::file_id(file_id))
                    .caption(formatted_text)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .await?;
            }
            Some("animation") => {
                bot.send_animation(chat.id, InputFile::file_id(file_id))
                    .caption(formatted_text)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .await?;
            }
            Some("sticker") => {
                // Send sticker first, then the message
                bot.send_sticker(chat.id, InputFile::file_id(file_id))
                    .await?;
                if !formatted_text.is_empty() {
                    bot.send_message(chat.id, formatted_text)
                        .parse_mode(ParseMode::Html)
                        .reply_markup(keyboard)
                        .await?;
                }
            }
            _ => {
                // Unknown media type, send as document or just text
                bot.send_message(chat.id, formatted_text)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .await?;
            }
        }
    } else {
        // Text only welcome
        bot.send_message(chat.id, formatted_text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }

    info!(
        "Sent welcome message to {} in chat {}",
        user.first_name, chat.id
    );

    Ok(())
}


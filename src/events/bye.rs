//! Goodbye event handler.
//!
//! Handles member leaves and sends customizable goodbye messages.

use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;
use teloxide::types::{ChatMemberUpdated, InputFile, ParseMode};
use tracing::{debug, info};

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::GroupSettingsRepo;
use crate::plugins::bye::{build_bye_keyboard, format_bye_text};

/// Returns the handler for member leave events.
pub fn handler() -> UpdateHandler<anyhow::Error> {
    dptree::filter(is_member_left).endpoint(bye_handler)
}

/// Check if this is a member leaving.
fn is_member_left(update: ChatMemberUpdated) -> bool {
    let old = &update.old_chat_member;
    let new = &update.new_chat_member;

    // Check if user was a member before and is no longer
    // Also exclude bots
    let is_leaving = old.is_present() && !new.is_present();
    let is_not_bot = !old.user.is_bot;

    is_leaving && is_not_bot
}

/// Handle member leave event.
async fn bye_handler(
    bot: ThrottledBot,
    update: ChatMemberUpdated,
    state: AppState,
) -> anyhow::Result<()> {
    let chat = update.chat;
    let user = &update.old_chat_member.user;

    debug!("Member {} left chat {}", user.id, chat.id);

    // Get group settings
    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let settings = repo.get_or_create(chat.id.0).await?;

    // Check if goodbye is enabled
    if !settings.bye.enabled {
        debug!("Goodbye disabled for chat {}", chat.id);
        return Ok(());
    }

    // Get goodbye message text
    let template = settings
        .bye
        .message
        .as_deref()
        .unwrap_or("👋 Selamat tinggal, {mention}!");

    let chat_title = chat.title().unwrap_or("Grup");

    // Format the goodbye text with placeholders
    let formatted_text = format_bye_text(template, user, chat_title);

    // Build keyboard if buttons are configured
    let keyboard = build_bye_keyboard(&settings.bye.buttons);

    // Send goodbye message (with or without media)
    if let Some(ref file_id) = settings.bye.media_file_id {
        match settings.bye.media_type.as_deref() {
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
                bot.send_message(chat.id, formatted_text)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .await?;
            }
        }
    } else {
        // Text only goodbye
        bot.send_message(chat.id, formatted_text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }

    info!(
        "Sent goodbye message for {} in chat {}",
        user.first_name, chat.id
    );

    Ok(())
}

//! Target resolution utilities for user commands.
//!
//! Provides shared functions for resolving target users from messages
//! via reply, user ID, TextMention, or @username.

use teloxide::prelude::*;
use teloxide::types::{Message, MessageEntityKind, UserId};

use crate::bot::dispatcher::{AppState, ThrottledBot};

/// Get target user from message (reply, ID, TextMention, @username).
/// Returns (user_id, first_name, skip_words_count for args after target).
///
/// Resolution order:
/// 1. Reply message → use `reply.from`
/// 2. ID argument → lookup via `UserRepo.get_by_id`
/// 3. TextMention entity → extract user from entity
/// 4. @username → lookup via `UserRepo.get_by_username`, fallback to `get_chat`
pub async fn get_target_from_msg(
    bot: &ThrottledBot,
    msg: &Message,
    state: &AppState,
) -> Option<(UserId, String, usize)> {
    // 1. Check reply
    if let Some(reply) = msg.reply_to_message() {
        if let Some(user) = &reply.from {
            return Some((user.id, user.first_name.clone(), 0));
        }
    }

    if let Some(text) = msg.text() {
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() > 1 {
            let arg = parts[1];

            // 2. Try ID
            if let Ok(id) = arg.parse::<u64>() {
                // Try to get name from UserRepo if available
                let name = if let Ok(Some(user)) = state.users.get_by_id(id).await {
                    user.first_name
                } else {
                    format!("User {}", id)
                };
                return Some((UserId(id), name, 1));
            }

            // 3. Try TextMention
            if let Some(entities) = msg.entities() {
                for entity in entities {
                    if let MessageEntityKind::TextMention { user } = &entity.kind {
                        // Only consider entities near the command (first 20 chars)
                        if entity.offset < 20 {
                            return Some((user.id, user.first_name.clone(), 1));
                        }
                    }
                }
            }

            // 4. Try @username via UserRepo
            if arg.starts_with('@') {
                let username = arg.trim_start_matches('@');
                if let Ok(Some(user)) = state.users.get_by_username(username).await {
                    return Some((UserId(user.user_id), user.first_name, 1));
                }
                // Fallback to get_chat (for bots/users not in our cache)
                if let Ok(chat) = bot.get_chat(arg.to_string()).await {
                    if chat.is_private() {
                        let name = chat.first_name().unwrap_or("User").to_string();
                        return Some((UserId(chat.id.0 as u64), name, 1));
                    }
                }
            }
        }
    }

    None
}

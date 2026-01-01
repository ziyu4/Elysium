//! Admin management commands.
//!
//! Commands for promoting and demoting group administrators.

use teloxide::prelude::*;
use teloxide::types::{ChatAdministratorRights, ParseMode, ReplyParameters, UserId};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::utils::html_escape;
use crate::i18n::get_text;

/// Handle /promote command - promote a user to admin.
///
/// Usage: /promote [@username | reply] [custom title]
pub async fn promote_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    // Must be in group
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, get_text(&locale, "admin.error_group_only"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Check permission: can_promote_members
    if !state.permissions.can_promote_members(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanPromoteMembers"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Get target user (reply or @username/ID)
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().skip(1).collect();

    let (target_user_id, custom_title) = if let Some(reply) = msg.reply_to_message() {
        // From reply
        let target_id = reply.from.as_ref().map(|u| u.id);
        let title = if parts.is_empty() { None } else { Some(parts.join(" ")) };
        (target_id, title)
    } else if !parts.is_empty() {
        // From argument
        let first_arg = parts[0];
        let target_id = parse_user_id(first_arg);
        let title = if parts.len() > 1 { Some(parts[1..].join(" ")) } else { None };
        (target_id, title)
    } else {
        bot.send_message(
            chat_id,
            get_text(&locale, "admin.promote_usage"),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    };

    let target_user_id = match target_user_id {
        Some(id) => id,
        None => {
            bot.send_message(chat_id, get_text(&locale, "admin.error_user_not_found"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    // Promote with default admin rights
    let rights = ChatAdministratorRights {
        is_anonymous: false,
        can_manage_chat: true,
        can_delete_messages: true,
        can_manage_video_chats: false,
        can_restrict_members: true,
        can_promote_members: false, // Can't promote others
        can_change_info: true,
        can_invite_users: true,
        can_post_messages: None,
        can_edit_messages: None,
        can_pin_messages: Some(true),
        can_post_stories: None,
        can_edit_stories: None,
        can_delete_stories: None,
        can_manage_topics: Some(false),
    };

    match bot.promote_chat_member(chat_id, target_user_id)
        .can_manage_chat(rights.can_manage_chat)
        .can_delete_messages(rights.can_delete_messages)
        .can_restrict_members(rights.can_restrict_members)
        .can_promote_members(rights.can_promote_members)
        .can_change_info(rights.can_change_info)
        .can_invite_users(rights.can_invite_users)
        .can_pin_messages(rights.can_pin_messages.unwrap_or(false))
        .await
    {
        Ok(_) => {
            info!("Promoted user {} in chat {}", target_user_id, chat_id);

            // Set custom title if provided
            if let Some(title) = &custom_title {
                let title = title.chars().take(16).collect::<String>(); // Max 16 chars
                let _ = bot.set_chat_administrator_custom_title(chat_id, target_user_id, &title).await;
            }

            let title_msg = custom_title
                .map(|t| format!(" ({})", html_escape(&t)))
                .unwrap_or_default();

            let success_text = get_text(&locale, "admin.promote_success")
                .replace("{user_id}", &target_user_id.to_string())
                .replace("{user_name}", &target_user_id.to_string())
                .replace("{title}", &title_msg);

            bot.send_message(
                chat_id,
                success_text,
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        }
        Err(e) => {
            bot.send_message(chat_id, get_text(&locale, "admin.promote_fail").replace("{error}", &e.to_string()))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
    }

    Ok(())
}

/// Handle /demote command - demote an admin to regular member.
///
/// Usage: /demote [@username | reply]
pub async fn demote_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    // Must be in group
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, get_text(&locale, "admin.error_group_only"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Check permission: can_promote_members
    if !state.permissions.can_promote_members(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanPromoteMembers"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Get target user
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().skip(1).collect();

    let target_user_id = if let Some(reply) = msg.reply_to_message() {
        reply.from.as_ref().map(|u| u.id)
    } else if !parts.is_empty() {
        parse_user_id(parts[0])
    } else {
        bot.send_message(
            chat_id,
            get_text(&locale, "admin.demote_usage"),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    };

    let target_user_id = match target_user_id {
        Some(id) => id,
        None => {
            bot.send_message(chat_id, get_text(&locale, "admin.error_user_not_found"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    // Demote by removing all admin rights
    match bot.promote_chat_member(chat_id, target_user_id)
        .can_manage_chat(false)
        .can_delete_messages(false)
        .can_restrict_members(false)
        .can_promote_members(false)
        .can_change_info(false)
        .can_invite_users(false)
        .can_pin_messages(false)
        .await
    {
        Ok(_) => {
            info!("Demoted user {} in chat {}", target_user_id, chat_id);

            // Invalidate permissions cache
            state.permissions.invalidate(chat_id, target_user_id);

            let success_text = get_text(&locale, "admin.demote_success")
                .replace("{user_id}", &target_user_id.to_string())
                .replace("{user_name}", &target_user_id.to_string());

            bot.send_message(
                chat_id,
                success_text,
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        }
        Err(e) => {
            bot.send_message(chat_id, get_text(&locale, "admin.demote_fail").replace("{error}", &e.to_string()))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
    }

    Ok(())
}

/// Parse user ID from string (@username or numeric ID).
fn parse_user_id(input: &str) -> Option<UserId> {
    // Try numeric ID first
    if let Ok(id) = input.parse::<u64>() {
        return Some(UserId(id));
    }

    // @username - can't resolve directly without getChat/getChatMember
    // For now, only support numeric IDs and reply
    None
}

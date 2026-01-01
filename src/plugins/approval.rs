//! Approval command handlers.
//!
//! Commands for managing approved users who bypass antiflood.

use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::i18n::get_text;


/// Handle /approve command - approve a user.
pub async fn approve_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id);

    if user_id.is_none() {
        return Ok(());
    }
    let admin_id = user_id.unwrap();

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        // Resolve locale (fall back to user lang as we don't have group context effectively if not a group, but safe to use 0)
        let locale = state.get_locale(Some(chat_id.0), Some(admin_id.0)).await;
        bot.send_message(chat_id, get_text(&locale, "approval.error_group_only"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Resolve locale
    let locale = state.get_locale(Some(chat_id.0), Some(admin_id.0)).await;

    // Check if user is admin
    if !state.permissions.is_admin(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(chat_id, get_text(&locale, "approval.error_admin_only"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Get target user from reply or args
    let target_user = get_target_user(&msg, &state, &bot).await?;

    let (target_id, target_name, _) = match target_user {
        Some(u) => u,
        None => {
            bot.send_message(
                chat_id,
                get_text(&locale, "approval.approve_usage"),
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
            return Ok(());
        }
    };

    let chat_title = msg.chat.title().unwrap_or("grup ini");

    if state.message_context.approve_user(chat_id.0, target_id).await? {
        let message = get_text(&locale, "approval.approved_success")
            .replace("{name}", &target_name)
            .replace("{chat}", &html_escape(chat_title));
            
        bot.send_message(chat_id, message)
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        info!("User {} approved in chat {} by {}", target_id, chat_id, admin_id);
    } else {
        bot.send_message(
            chat_id, 
            get_text(&locale, "approval.already_approved")
                .replace("{name}", &target_name)
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    }

    Ok(())
}

/// Handle /unapprove command - remove approval.
pub async fn unapprove_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let admin_id = match msg.from.as_ref() {
        Some(u) => u.id,
        None => return Ok(()),
    };

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Resolve locale
    let locale = state.get_locale(Some(chat_id.0), Some(admin_id.0)).await;

    if !state.permissions.is_admin(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(chat_id, get_text(&locale, "approval.error_admin_only"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let target_user = get_target_user(&msg, &state, &bot).await?;

    let (target_id, target_name, _) = match target_user {
        Some(u) => u,
        None => {
            bot.send_message(
                chat_id,
                get_text(&locale, "approval.unapprove_usage"),
            )
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
            return Ok(());
        }
    };

    if state.message_context.unapprove_user(chat_id.0, target_id).await? {
        let message = get_text(&locale, "approval.unapproved_success")
            .replace("{name}", &target_name);

        bot.send_message(chat_id, message)
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        info!("User {} unapproved in chat {} by {}", target_id, chat_id, admin_id);
    } else {
        bot.send_message(
            chat_id, 
            get_text(&locale, "approval.not_approved")
                .replace("{name}", &target_name)
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    }

    Ok(())
}

/// Handle /unapproveall command - clear all approvals.
pub async fn unapproveall_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let admin_id = match msg.from.as_ref() {
        Some(u) => u.id,
        None => return Ok(()),
    };

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Resolve locale
    let locale = state.get_locale(Some(chat_id.0), Some(admin_id.0)).await;

    // Requires can_promote_members (higher level admin)
    if !state.permissions.can_promote_members(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "approval.error_promote_permission"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    let count = state.message_context.unapprove_all(chat_id.0).await?;

    bot.send_message(
        chat_id,
        get_text(&locale, "approval.unapprove_all_success")
            .replace("{count}", &count.to_string()),
    )
    .parse_mode(ParseMode::Html)
    .reply_parameters(ReplyParameters::new(msg.id))
    .await?;

    info!("All {} users unapproved in chat {} by {}", count, chat_id, admin_id);
    Ok(())
}

/// Handle /approval command - check if user is approved.
pub async fn approval_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, "⚠️ Perintah ini hanya untuk grup.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let locale = state.get_locale(Some(chat_id.0), Some(user.id.0)).await;
    let ctx = state.message_context.get_or_default(chat_id.0).await?;

    let status = if ctx.is_approved(user.id.0) {
        get_text(&locale, "approval.status_approved")
    } else {
        get_text(&locale, "approval.status_not_approved")
    };

    bot.send_message(chat_id, status)
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    Ok(())
}

/// Handle /approved or /approvals command - list approved users.
pub async fn approved_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    let locale = state.get_locale(Some(chat_id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;
    let ctx = state.message_context.get_or_default(chat_id.0).await?;

    if ctx.approved_users.is_empty() {
        bot.send_message(chat_id, get_text(&locale, "approval.list_empty"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Build list of approved users
    let mut list = get_text(&locale, "approval.list_header");
    for (i, user_id) in ctx.approved_users.iter().enumerate() {
        list.push_str(&format!(
            "{}. <a href=\"tg://user?id={}\">{}</a>\n",
            i + 1,
            user_id,
            user_id
        ));
    }
    list.push_str(&get_text(&locale, "approval.list_footer").replace("{count}", &ctx.approved_users.len().to_string()));

    bot.send_message(chat_id, list)
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

/// Get target user from reply or command args.
/// Returns (user_id, mention_html, first_name)
async fn get_target_user(
    msg: &Message,
    _state: &AppState,
    _bot: &ThrottledBot,
) -> anyhow::Result<Option<(u64, String, String)>> {
    // Check reply first
    if let Some(reply) = msg.reply_to_message()
        && let Some(user) = reply.from.as_ref() {
            let mention = format!(
                "<a href=\"tg://user?id={}\">{}</a>",
                user.id,
                html_escape(&user.first_name)
            );
            return Ok(Some((user.id.0, mention, user.first_name.clone())));
        }

    // Check text args for user ID
    if let Some(text) = msg.text() {
        let args: Vec<&str> = text.split_whitespace().skip(1).collect();
        if let Some(arg) = args.first() {
            // Try to parse as user ID
            if let Ok(id) = arg.parse::<u64>() {
                let mention = format!("<a href=\"tg://user?id={}\">{}</a>", id, id);
                return Ok(Some((id, mention, id.to_string())));
            }
        }
    }

    Ok(None)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

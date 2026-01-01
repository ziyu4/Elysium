//! Ban management commands.
//!
//! Commands for banning, unbanning, and kicking users.

use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters, UserId};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::utils::{html_escape, parse_duration};
use crate::i18n::get_text;

/// Handle /ban command.
pub async fn ban_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    ban_action(bot, msg, state, BanMode::Forever).await
}

/// Handle /tban command.
pub async fn tban_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    ban_action(bot, msg, state, BanMode::Temporary).await
}

/// Handle /dban command.
pub async fn dban_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    ban_action(bot, msg, state, BanMode::DeleteAndBan).await
}

/// Handle /sban command - silent ban.
pub async fn sban_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    ban_action(bot, msg, state, BanMode::SilentBan).await
}

/// Handle /kick command.
pub async fn kick_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    ban_action(bot, msg, state, BanMode::Kick).await
}

/// Handle /dkick command - delete and kick.
pub async fn dkick_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    ban_action(bot, msg, state, BanMode::DeleteKick).await
}

/// Handle /skick command - silent kick.
pub async fn skick_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    ban_action(bot, msg, state, BanMode::SilentKick).await
}

/// Handle /unban command.
pub async fn unban_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    ban_action(bot, msg, state, BanMode::Unban).await
}

/// Handle /kickme command - user kicks themselves.
pub async fn kickme_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    if user_id == UserId(0) {
        return Ok(());
    }

    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    // Ban then unban = kick
    match bot.ban_chat_member(chat_id, user_id).await {
        Ok(_) => {
            let _ = bot.unban_chat_member(chat_id, user_id).await;
            bot.send_message(chat_id, get_text(&locale, "ban.kickme_goodbye"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
        Err(e) => {
            bot.send_message(chat_id, get_text(&locale, "ban.error_kick_failed").replace("{error}", &e.to_string()))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
    }

    Ok(())
}

#[derive(PartialEq, Clone, Copy)]
enum BanMode {
    Forever,
    Temporary,
    DeleteAndBan,
    SilentBan,
    Kick,
    DeleteKick,
    SilentKick,
    Unban,
}

async fn ban_action(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    mode: BanMode,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Check permission: can_restrict_members
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;
    if !state.permissions.can_restrict_members(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            get_text(&locale, "common.error_missing_permission")
                .replace("{permission}", "CanRestrictMembers"),
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Parse arguments
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().skip(1).collect();

    // Determine target - now also capture first_name
    let (target_id, target_name, reason_start_idx) = if let Some(reply) = msg.reply_to_message() {
        if let Some(user) = &reply.from {
            (Some(user.id), user.first_name.clone(), 0) // Args start at 0 if reply
        } else {
            (None, String::new(), 0)
        }
    } else if !parts.is_empty() {
        // Try parsing first arg as ID
        if let Ok(id) = parts[0].parse::<u64>() {
            // When using ID, we don't have the name - just use "User"
            (Some(UserId(id)), format!("User {}", id), 1)
        } else {
            (None, String::new(), 0)
        }
    } else {
        (None, String::new(), 0)
    };

    let target_id = match target_id {
        Some(id) => id,
        None => {
            bot.send_message(chat_id, get_text(&locale, "ban.error_user_not_found"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    // Anti-Admin Check (except for Unban)
    if mode != BanMode::Unban
        && state.permissions.is_admin(chat_id, target_id).await.unwrap_or(false) {
            let action_text = match mode {
                BanMode::Forever | BanMode::Temporary | BanMode::DeleteAndBan | BanMode::SilentBan => {
                    get_text(&locale, "ban.action_ban")
                }
                BanMode::Kick | BanMode::DeleteKick | BanMode::SilentKick => get_text(&locale, "ban.action_kick"),
                BanMode::Unban => unreachable!(),
            };
            bot.send_message(
                chat_id,
                get_text(&locale, "ban.anti_admin").replace("{action}", &action_text)
            )
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }

    // Handle Time for Tban
    let (duration, reason_idx) = if mode == BanMode::Temporary {
        if parts.len() > reason_start_idx {
            if let Some(d) = parse_duration(parts[reason_start_idx]) {
                (Some(d), reason_start_idx + 1)
            } else {
                bot.send_message(chat_id, get_text(&locale, "ban.error_time_format"))
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
                return Ok(());
            }
        } else {
             bot.send_message(chat_id, get_text(&locale, "ban.error_duration_missing"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    } else {
        (None, reason_start_idx)
    };

    // Extract Reason - None if not provided
    let reason = if parts.len() > reason_idx {
        let r = parts[reason_idx..].join(" ");
        if r.is_empty() { None } else { Some(r) }
    } else {
        None
    };

    // Format reason line - empty if no reason
    let reason_line = reason.as_ref()
        .map(|r| get_text(&locale, "ban.reason").replace("{reason}", &html_escape(r)))
        .unwrap_or_default();

    // For silent modes, delete command message first
    let is_silent = matches!(mode, BanMode::SilentBan | BanMode::SilentKick);
    if is_silent {
        let _ = bot.delete_message(chat_id, msg.id).await;
    }

    // Execute
    match mode {
        BanMode::Forever => {
            bot.ban_chat_member(chat_id, target_id)
                .await?;
            bot.send_message(chat_id, get_text(&locale, "ban.banned")
                .replace("{id}", &target_id.to_string())
                .replace("{name}", &html_escape(&target_name))
                .replace("{reason}", &reason_line)
            ).parse_mode(ParseMode::Html).await?;
        },
        BanMode::Temporary => {
            let d = duration.unwrap();
            let until = SystemTime::now() + d;
            let until_date = until.duration_since(UNIX_EPOCH)?.as_secs();
            let until_dt = chrono::DateTime::from_timestamp(until_date as i64, 0).unwrap_or_default();
            
            bot.ban_chat_member(chat_id, target_id)
                .until_date(until_dt)
                .await?;

            bot.send_message(chat_id, get_text(&locale, "ban.tban")
                .replace("{id}", &target_id.to_string())
                .replace("{name}", &html_escape(&target_name))
                .replace("{duration}", &format!("{:?}", d)) // Ideally format_duration
                .replace("{reason}", &reason_line)
            ).parse_mode(ParseMode::Html).await?;
        },
        BanMode::DeleteAndBan => {
            if let Some(reply) = msg.reply_to_message() {
                 let _ = bot.delete_message(chat_id, reply.id).await;
            }
            bot.ban_chat_member(chat_id, target_id).await?;
            
            bot.send_message(chat_id, get_text(&locale, "ban.dban")
                .replace("{id}", &target_id.to_string())
                .replace("{name}", &html_escape(&target_name))
                .replace("{reason}", &reason_line)
            ).parse_mode(ParseMode::Html).await?;
        },
        BanMode::SilentBan => {
            // Silent - no message, command already deleted
            bot.ban_chat_member(chat_id, target_id).await?;
        },
        BanMode::Kick => {
            // Ban then Unban
            bot.ban_chat_member(chat_id, target_id).await?;
            bot.unban_chat_member(chat_id, target_id).await?;
            
            bot.send_message(chat_id, get_text(&locale, "ban.kicked")
                .replace("{id}", &target_id.to_string())
                .replace("{name}", &html_escape(&target_name))
                .replace("{reason}", &reason_line)
            ).parse_mode(ParseMode::Html).await?;
        },
        BanMode::DeleteKick => {
            if let Some(reply) = msg.reply_to_message() {
                let _ = bot.delete_message(chat_id, reply.id).await;
            }
            bot.ban_chat_member(chat_id, target_id).await?;
            bot.unban_chat_member(chat_id, target_id).await?;
            
            bot.send_message(chat_id, get_text(&locale, "ban.dkick")
                .replace("{id}", &target_id.to_string())
                .replace("{name}", &html_escape(&target_name))
                .replace("{reason}", &reason_line)
            ).parse_mode(ParseMode::Html).await?;
        },
        BanMode::SilentKick => {
            // Silent - no message, command already deleted
            bot.ban_chat_member(chat_id, target_id).await?;
            bot.unban_chat_member(chat_id, target_id).await?;
        },
        BanMode::Unban => {
             bot.unban_chat_member(chat_id, target_id).await?;
             bot.send_message(chat_id, get_text(&locale, "ban.unbanned")
                .replace("{id}", &target_id.to_string())
                .replace("{name}", &html_escape(&target_name))
            ).parse_mode(ParseMode::Html).await?;
        },
    }

    Ok(())
}

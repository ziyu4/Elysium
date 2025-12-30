//! Mute management commands.
//!
//! Commands for muting and unmuting users.

use teloxide::prelude::*;
use teloxide::types::{ChatPermissions, ParseMode, ReplyParameters, UserId};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::utils::{html_escape, parse_duration};

/// Handle /mute command - now supports optional duration.
/// /mute @user = mute forever
/// /mute @user 2h = mute for 2 hours
pub async fn mute_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    mute_action(bot, msg, state, MuteMode::Normal).await
}

/// Handle /tmute command.
pub async fn tmute_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    mute_action(bot, msg, state, MuteMode::Temporary).await
}

/// Handle /dmute command - delete and mute.
pub async fn dmute_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    mute_action(bot, msg, state, MuteMode::DeleteMute).await
}

/// Handle /smute command - silent mute.
pub async fn smute_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    mute_action(bot, msg, state, MuteMode::SilentMute).await
}

/// Handle /unmute command.
pub async fn unmute_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    mute_action(bot, msg, state, MuteMode::Unmute).await
}

#[derive(PartialEq, Clone, Copy)]
enum MuteMode {
    Normal,     // /mute - optional duration (default forever)
    Temporary,  // /tmute - requires duration
    DeleteMute, // /dmute - delete replied message
    SilentMute, // /smute - silent, delete command
    Unmute,
}

async fn mute_action(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    mode: MuteMode,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Permission check
    if !state.permissions.can_restrict_members(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk membatasi member.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Extract target
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().skip(1).collect();

    // Determine target - now also capture first_name
    let (target_id, target_name, reason_start_idx) = if let Some(reply) = msg.reply_to_message() {
        if let Some(user) = &reply.from {
            (Some(user.id), user.first_name.clone(), 0)
        } else {
            (None, String::new(), 0)
        }
    } else if !parts.is_empty() {
        if let Ok(id) = parts[0].parse::<u64>() {
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
            bot.send_message(chat_id, "❌ User tidak ditemukan.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    // Anti-Admin
    if mode != MuteMode::Unmute
        && state.permissions.is_admin(chat_id, target_id).await.unwrap_or(false) {
            bot.send_message(
                chat_id,
                "😏 Kenapa saya harus membisukan seorang admin? Sepertinya itu bukan ide yang bagus."
            )
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }

    // For silent modes, delete command message first
    if mode == MuteMode::SilentMute {
        let _ = bot.delete_message(chat_id, msg.id).await;
    }

    // Mode handling
    match mode {
        MuteMode::Normal | MuteMode::Temporary | MuteMode::DeleteMute | MuteMode::SilentMute => {
            // For Normal mode: duration is optional (default forever)
            // For Temporary mode: duration is required
            // For DeleteMute/SilentMute: behavior like Normal (optional duration)
            
            let requires_duration = mode == MuteMode::Temporary;
            
            let (until_dt, display_duration, reason_idx) = if parts.len() > reason_start_idx {
                // Try to parse duration from first available arg
                if let Some(d) = parse_duration(parts[reason_start_idx]) {
                    let until = SystemTime::now() + d;
                    let until_date = until.duration_since(UNIX_EPOCH)?.as_secs();
                    let dt = chrono::DateTime::from_timestamp(until_date as i64, 0).unwrap_or_default();
                    (Some(dt), Some(d), reason_start_idx + 1)
                } else if requires_duration {
                    // /tmute requires duration but got invalid format
                    bot.send_message(chat_id, "❌ Format waktu salah. Contoh: 1h, 30m, 1d")
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                    return Ok(());
                } else {
                    // /mute, /dmute, /smute - no duration = forever, treat as reason
                    (None, None, reason_start_idx)
                }
            } else if requires_duration {
                // /tmute without any args after target
                bot.send_message(chat_id, "❌ Tentukan durasi untuk temp mute. Contoh: /tmute @user 1h")
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
                return Ok(());
            } else {
                (None, None, reason_start_idx)
            };

            // Extract reason - None if not provided
            let reason = if parts.len() > reason_idx {
                let r = parts[reason_idx..].join(" ");
                if r.is_empty() { None } else { Some(r) }
            } else {
                None
            };

            // Delete replied message for DeleteMute
            if mode == MuteMode::DeleteMute
                && let Some(reply) = msg.reply_to_message() {
                    let _ = bot.delete_message(chat_id, reply.id).await;
                }

            // Mute permissions
            let permissions = ChatPermissions::empty(); // No rights = Muted

            let req = bot.restrict_chat_member(chat_id, target_id, permissions);
            let req = if let Some(dt) = until_dt {
                req.until_date(dt)
            } else {
                req
            };

            req.await?;

            // Don't send message for silent mode
            if mode != MuteMode::SilentMute {
                let duration_msg = display_duration.map(|d| format!("\nDurasi: {:?}", d)).unwrap_or_default();
                let reason_line = reason.as_ref()
                    .map(|r| format!("\nAlasan: {}", html_escape(r)))
                    .unwrap_or_default();
                
                let action_text = if mode == MuteMode::DeleteMute {
                    "dimute dan pesan dihapus"
                } else {
                    "dimute"
                };

                bot.send_message(chat_id, format!(
                    "😶 <a href=\"tg://user?id={}\">{}</a> {}.{}{}",
                    target_id, html_escape(&target_name), action_text, duration_msg, reason_line
                )).parse_mode(ParseMode::Html).await?;
            }
        },
        MuteMode::Unmute => {
            // Lift restrictions = Unmute
            let permissions = ChatPermissions::empty()
                | ChatPermissions::SEND_MESSAGES
                | ChatPermissions::SEND_AUDIOS
                | ChatPermissions::SEND_DOCUMENTS
                | ChatPermissions::SEND_PHOTOS
                | ChatPermissions::SEND_VIDEOS
                | ChatPermissions::SEND_VIDEO_NOTES
                | ChatPermissions::SEND_VOICE_NOTES
                | ChatPermissions::SEND_POLLS
                | ChatPermissions::SEND_OTHER_MESSAGES
                | ChatPermissions::ADD_WEB_PAGE_PREVIEWS
                | ChatPermissions::CHANGE_INFO
                | ChatPermissions::INVITE_USERS
                | ChatPermissions::PIN_MESSAGES
                | ChatPermissions::MANAGE_TOPICS;
            
            bot.restrict_chat_member(chat_id, target_id, permissions).await?;

            bot.send_message(chat_id, format!(
                "🔊 <a href=\"tg://user?id={}\">{}</a> diunmute.",
                target_id, html_escape(&target_name)
            )).parse_mode(ParseMode::Html).await?;
        }
    }

    Ok(())
}

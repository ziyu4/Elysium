//! Antiflood command handlers.
//!
//! Commands for configuring antiflood protection in groups.


use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::FloodPenalty;
use crate::i18n::get_text;

/// Handle /antiflood command - show or toggle antiflood.
pub async fn antiflood_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(user) => user.id,
        None => return Ok(()),
    };

    // Check if in group
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;
        bot.send_message(chat_id, get_text(&locale, "antiflood.error_group_only"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }
    
    // Resolve locale
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    // Check admin permission (can_change_info)
    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, get_text(&locale, "antiflood.error_permission"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let mut ctx = state.message_context.get_or_default(chat_id.0).await?;

    // Parse arguments
    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    if args.is_empty() {
        // Show current status
        let status = if ctx.antiflood.enabled {
            get_text(&locale, "antiflood.status_enabled")
                .replace("{limit}", &ctx.antiflood.max_messages.to_string())
                .replace("{seconds}", &ctx.antiflood.time_window_secs.to_string())
                .replace("{warns}", &ctx.antiflood.warnings_before_penalty.to_string())
                .replace("{penalty}", &penalty_to_string(&ctx.antiflood.penalty, &locale))
                .replace("{duration}", &duration_to_string(ctx.antiflood.penalty_duration_secs, &locale))
        } else {
            get_text(&locale, "antiflood.status_disabled")
        };

        bot.send_message(chat_id, status)
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    match args[0].to_lowercase().as_str() {
        "on" | "enable" => {
            ctx.antiflood.enabled = true;
            state.message_context.update_antiflood(chat_id.0, ctx.antiflood).await?;
            bot.send_message(chat_id, get_text(&locale, "antiflood.enabled"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Antiflood enabled in chat {}", chat_id);
        }
        "off" | "disable" => {
            ctx.antiflood.enabled = false;
            state.message_context.update_antiflood(chat_id.0, ctx.antiflood).await?;
            bot.send_message(chat_id, get_text(&locale, "antiflood.disabled"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Antiflood disabled in chat {}", chat_id);
        }
        _ => {
            bot.send_message(
                chat_id,
                get_text(&locale, "antiflood.usage")
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        }
    }

    Ok(())
}

/// Handle /setflood command - set flood limits.
pub async fn setflood_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(user) => user.id,
        None => return Ok(()),
    };

    // Check permissions
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, get_text(&locale, "antiflood.error_permission"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    if args.len() < 2 {
        bot.send_message(
            chat_id,
            get_text(&locale, "antiflood.setflood_usage"),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    let max_messages: u32 = match args[0].parse() {
        Ok(n) if (2..=100).contains(&n) => n,
        _ => {
            bot.send_message(chat_id, get_text(&locale, "antiflood.error_limit_count"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    let time_window: u32 = match args[1].parse() {
        Ok(n) if (1..=300).contains(&n) => n,
        _ => {
            bot.send_message(chat_id, get_text(&locale, "antiflood.error_limit_time"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    let mut ctx = state.message_context.get_or_default(chat_id.0).await?;
    ctx.antiflood.max_messages = max_messages;
    ctx.antiflood.time_window_secs = time_window;
    state.message_context.update_antiflood(chat_id.0, ctx.antiflood).await?;

    bot.send_message(
        chat_id,
        get_text(&locale, "antiflood.limit_set")
            .replace("{count}", &max_messages.to_string())
            .replace("{seconds}", &time_window.to_string()),
    )
    .parse_mode(ParseMode::Html)
    .reply_parameters(ReplyParameters::new(msg.id))
    .await?;

    Ok(())
}

/// Handle /setfloodpenalty command.
pub async fn setfloodpenalty_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(user) => user.id,
        None => return Ok(()),
    };

    // Resolve locale
    let locale = state.get_locale(Some(chat_id.0), Some(user_id.0)).await;

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, get_text(&locale, "antiflood.error_permission"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    if args.is_empty() {
        bot.send_message(
            chat_id,
            get_text(&locale, "antiflood.setpenalty_usage"),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    let penalty = match args[0].to_lowercase().as_str() {
        "warn" => FloodPenalty::Warn,
        "mute" => FloodPenalty::Mute,
        "kick" => FloodPenalty::Kick,
        "tempban" => FloodPenalty::TempBan,
        "ban" => FloodPenalty::Ban,
        _ => {
            bot.send_message(chat_id, get_text(&locale, "antiflood.error_penalty_type"))
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    let duration_secs = if args.len() > 1 {
        parse_duration(args[1]).unwrap_or(300)
    } else {
        300 // Default 5 minutes
    };

    let mut ctx = state.message_context.get_or_default(chat_id.0).await?;
    ctx.antiflood.penalty = penalty.clone();
    ctx.antiflood.penalty_duration_secs = duration_secs;
    state.message_context.update_antiflood(chat_id.0, ctx.antiflood.clone()).await?;

    bot.send_message(
        chat_id,
        get_text(&locale, "antiflood.penalty_set")
            .replace("{penalty}", &penalty_to_string(&penalty, &locale))
            .replace("{duration}", &duration_to_string(duration_secs, &locale)),
    )
    .parse_mode(ParseMode::Html)
    .reply_parameters(ReplyParameters::new(msg.id))
    .await?;

    Ok(())
}

/// Parse duration string like "5m", "1h", "30s" to seconds.
fn parse_duration(s: &str) -> Option<u64> {
    let s = s.to_lowercase();
    if let Some(num) = s.strip_suffix('s') {
        return num.parse().ok();
    }
    if let Some(num) = s.strip_suffix('m') {
        return num.parse::<u64>().ok().map(|n| n * 60);
    }
    if let Some(num) = s.strip_suffix('h') {
        return num.parse::<u64>().ok().map(|n| n * 3600);
    }
    if let Some(num) = s.strip_suffix('d') {
        return num.parse::<u64>().ok().map(|n| n * 86400);
    }
    s.parse().ok()
}

fn penalty_to_string(penalty: &FloodPenalty, locale: &str) -> String {
    let key = match penalty {
        FloodPenalty::Warn => "antiflood.penalty_warn",
        FloodPenalty::Mute => "antiflood.penalty_mute",
        FloodPenalty::Kick => "antiflood.penalty_kick",
        FloodPenalty::TempBan => "antiflood.penalty_tempban",
        FloodPenalty::Ban => "antiflood.penalty_ban",
    };
    get_text(locale, key)
}

fn duration_to_string(secs: u64, locale: &str) -> String {
    if secs == 0 {
        return get_text(locale, "antiflood.duration_permanent");
    }
    if secs < 60 {
        get_text(locale, "antiflood.duration_seconds")
            .replace("{seconds}", &secs.to_string())
    } else if secs < 3600 {
        get_text(locale, "antiflood.duration_minutes")
            .replace("{minutes}", &(secs / 60).to_string())
    } else if secs < 86400 {
        get_text(locale, "antiflood.duration_hours")
            .replace("{hours}", &(secs / 3600).to_string())
    } else {
        get_text(locale, "antiflood.duration_days")
            .replace("{days}", &(secs / 86400).to_string())
    }
}

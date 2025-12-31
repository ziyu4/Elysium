//! Antiflood command handlers.
//!
//! Commands for configuring antiflood protection in groups.


use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::FloodPenalty;

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
        bot.send_message(chat_id, "⚠️ Perintah ini hanya untuk grup.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Check admin permission (can_change_info)
    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, "❌ Anda harus menjadi admin dengan izin 'Ubah Info Grup'.")
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
            format!(
                "✅ <b>Antiflood Aktif</b>\n\n\
                📊 Limit: <code>{}</code> pesan dalam <code>{}</code> detik\n\
                ⚠️ Peringatan sebelum aksi: <code>{}</code>\n\
                🔨 Hukuman: {}\n\
                ⏱️ Durasi: {}",
                ctx.antiflood.max_messages,
                ctx.antiflood.time_window_secs,
                ctx.antiflood.warnings_before_penalty,
                penalty_to_string(&ctx.antiflood.penalty),
                duration_to_string(ctx.antiflood.penalty_duration_secs)
            )
        } else {
            "❌ <b>Antiflood Nonaktif</b>\n\nGunakan <code>/antiflood on</code> untuk mengaktifkan.".to_string()
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
            bot.send_message(chat_id, "✅ Antiflood diaktifkan!")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Antiflood enabled in chat {}", chat_id);
        }
        "off" | "disable" => {
            ctx.antiflood.enabled = false;
            state.message_context.update_antiflood(chat_id.0, ctx.antiflood).await?;
            bot.send_message(chat_id, "❌ Antiflood dinonaktifkan!")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            info!("Antiflood disabled in chat {}", chat_id);
        }
        _ => {
            bot.send_message(
                chat_id,
                "📖 <b>Penggunaan Antiflood</b>\n\n\
                <code>/antiflood</code> - Lihat status\n\
                <code>/antiflood on</code> - Aktifkan\n\
                <code>/antiflood off</code> - Nonaktifkan\n\
                <code>/setflood &lt;jumlah&gt; &lt;detik&gt;</code> - Atur limit\n\
                <code>/setfloodpenalty &lt;warn/mute/kick/ban&gt;</code> - Atur hukuman"
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

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    if args.len() < 2 {
        bot.send_message(
            chat_id,
            "📖 <b>Penggunaan:</b>\n<code>/setflood &lt;jumlah_pesan&gt; &lt;detik&gt;</code>\n\n\
            Contoh: <code>/setflood 5 10</code> (5 pesan dalam 10 detik)",
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    let max_messages: u32 = match args[0].parse() {
        Ok(n) if (2..=100).contains(&n) => n,
        _ => {
            bot.send_message(chat_id, "❌ Jumlah pesan harus antara 2-100.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    let time_window: u32 = match args[1].parse() {
        Ok(n) if (1..=300).contains(&n) => n,
        _ => {
            bot.send_message(chat_id, "❌ Waktu harus antara 1-300 detik.")
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
        format!(
            "✅ Limit flood diatur: <b>{}</b> pesan dalam <b>{}</b> detik",
            max_messages, time_window
        ),
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

    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    if args.is_empty() {
        bot.send_message(
            chat_id,
            "📖 <b>Penggunaan:</b>\n<code>/setfloodpenalty &lt;tipe&gt; [durasi]</code>\n\n\
            Tipe: <code>warn</code>, <code>mute</code>, <code>kick</code>, <code>tempban</code>, <code>ban</code>\n\
            Durasi (untuk mute/tempban): dalam detik atau format 1h, 30m, dll",
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
            bot.send_message(chat_id, "❌ Tipe tidak valid. Gunakan: warn, mute, kick, tempban, ban")
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
        format!(
            "✅ Hukuman flood diatur: <b>{}</b> ({})",
            penalty_to_string(&penalty),
            duration_to_string(duration_secs)
        ),
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

fn penalty_to_string(penalty: &FloodPenalty) -> &'static str {
    match penalty {
        FloodPenalty::Warn => "⚠️ Peringatan",
        FloodPenalty::Mute => "🔇 Mute",
        FloodPenalty::Kick => "👢 Kick",
        FloodPenalty::TempBan => "⏳ Ban Sementara",
        FloodPenalty::Ban => "🔨 Ban Permanen",
    }
}

fn duration_to_string(secs: u64) -> String {
    if secs == 0 {
        return "Permanen".to_string();
    }
    if secs < 60 {
        format!("{} detik", secs)
    } else if secs < 3600 {
        format!("{} menit", secs / 60)
    } else if secs < 86400 {
        format!("{} jam", secs / 3600)
    } else {
        format!("{} hari", secs / 86400)
    }
}

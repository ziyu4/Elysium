//! Warning command handlers.
//!
//! Commands for managing user warnings in groups.

use teloxide::prelude::*;
use teloxide::types::{
    ChatId, ChatPermissions, InlineKeyboardButton, InlineKeyboardMarkup, MessageEntityKind, ParseMode,
    ReplyParameters, UserId,
};
use tracing::info;

use crate::database::WarnMode;
use crate::utils::parser::format_duration_full as format_duration;
use crate::utils::{html_escape, parse_duration};

use crate::bot::dispatcher::{AppState, ThrottledBot};

/// Handle /warn command.
pub async fn warn_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    warn_action(bot, msg, state, WarnAction::Normal).await
}

/// Handle /dwarn command - warn and delete message.
pub async fn dwarn_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    warn_action(bot, msg, state, WarnAction::DeleteMsg).await
}

/// Handle /swarn command - silent warn.
pub async fn swarn_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    warn_action(bot, msg, state, WarnAction::Silent).await
}

#[derive(PartialEq, Clone, Copy)]
enum WarnAction {
    Normal,
    DeleteMsg,  // /dwarn - delete their message
    Silent,     // /swarn - delete command
}

async fn warn_action(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    action: WarnAction,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let admin_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Permission check
    if !state.permissions.can_restrict_members(chat_id, admin_id).await.unwrap_or(false) {
        if action != WarnAction::Silent {
            bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk membatasi member.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
        return Ok(());
    }

    // Get target
    let (target_id, target_name, skip_words) = match get_target_from_msg(&bot, &msg, &state).await {
        Some(t) => t,
        None => {
            if action != WarnAction::Silent {
                bot.send_message(chat_id, "❌ Siapa yang mau di warn?")
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
            return Ok(());
        }
    };

    // Anti-Admin check
    if state.permissions.is_admin(chat_id, target_id).await.unwrap_or(false) {
        if action != WarnAction::Silent {
            bot.send_message(chat_id, "❌ Maaf, aku terlalu malash untuk memperingatkan admin.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
        return Ok(());
    }

    // Extract reason
    let text = msg.text().unwrap_or("");
    let reason = text
        .split_whitespace()
        .skip(1 + skip_words) // skip cmd + skip_words
        .collect::<Vec<_>>()
        .join(" ");
    let reason = if reason.is_empty() {
        None
    } else {
        Some(reason)
    };

    // Delete messages based on action
    if action == WarnAction::DeleteMsg {
        if let Some(reply) = msg.reply_to_message() {
            let _ = bot.delete_message(chat_id, reply.id).await;
        }
    }
    if action == WarnAction::Silent {
        let _ = bot.delete_message(chat_id, msg.id).await;
    }

    // Add warning using WarnsRepository
    // Note: add_warning returns the NEW count
    let warn_count = state.warns.add_warning(
        chat_id.0, 
        target_id.0, 
        reason.clone(), 
        admin_id.0
    ).await?;

    // Check limit
    // Need to fetch config to know limit
    let warns_data = state.warns.get_or_create(chat_id.0).await?;
    let limit = warns_data.config.limit;

    if warn_count as u32 >= limit {
        // Apply penalty
        let penalty_result = apply_warn_penalty(
            &bot,
            chat_id,
            target_id,
            &target_name,
            &warns_data.config.mode,
            warns_data.config.action_duration_secs,
        ).await;

        // Clear user warnings after penalty
        state.warns.reset_warnings(chat_id.0, target_id.0).await?;

        if action != WarnAction::Silent {
            let penalty_msg = match penalty_result {
                Ok(msg) => msg,
                Err(_) => "❌ Gagal menerapkan hukuman.".to_string(),
            };
            bot.send_message(chat_id, format!(
                "⚠️ <a href=\"tg://user?id={}\">{}</a> telah mencapai batas peringatan ({}/{})!\n\n{}",
                target_id,
                html_escape(&target_name),
                warn_count,
                limit,
                penalty_msg
            ))
            .parse_mode(ParseMode::Html)
            .await?;
        }

        info!("User {} reached warn limit in chat {}, penalty applied", target_id, chat_id);
    } else {
        // Just warning
        if action != WarnAction::Silent {
            let reason_text = reason.as_deref().unwrap_or("Tidak ada alasan");
            let keyboard = InlineKeyboardMarkup::new(vec![vec![
                InlineKeyboardButton::callback(
                    "🗑 Hapus peringatan (khusus admin)",
                    format!("warn_remove:{}:{}", chat_id.0, target_id.0),
                ),
            ]]);

            bot.send_message(
                chat_id,
                format!(
                    "⚠️ <a href=\"tg://user?id={}\">{}</a> memiliki {}/{} peringatan; hati-hati!\n\n<b>Alasan:</b> {}",
                    target_id,
                    html_escape(&target_name),
                    warn_count,
                    limit,
                    html_escape(reason_text)
                ),
            )
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
        }

        info!("User {} warned in chat {} ({}/{})", target_id, chat_id, warn_count, limit);
    }

    Ok(())
}

/// Apply penalty based on warn mode.
async fn apply_warn_penalty(
    bot: &ThrottledBot,
    chat_id: ChatId,
    user_id: UserId,
    user_name: &str,
    mode: &WarnMode,
    duration_secs: u64,
) -> anyhow::Result<String> {
    match mode {
        WarnMode::Ban => {
            bot.ban_chat_member(chat_id, user_id).await?;
            Ok(format!("🔨 {} telah di-ban permanen.", html_escape(user_name)))
        }
        WarnMode::Mute => {
            let perms = ChatPermissions::empty();
            let until = chrono::Utc::now() + chrono::Duration::days(366);
            bot.restrict_chat_member(chat_id, user_id, perms)
                .until_date(until)
                .await?;
            Ok(format!("🔇 {} telah di-mute permanen.", html_escape(user_name)))
        }
        WarnMode::Kick => {
            bot.ban_chat_member(chat_id, user_id).await?;
            let _ = bot.unban_chat_member(chat_id, user_id).await;
            Ok(format!("👢 {} telah dikeluarkan.", html_escape(user_name)))
        }
        WarnMode::TBan => {
            let until = chrono::Utc::now() + chrono::Duration::seconds(duration_secs as i64);
            bot.ban_chat_member(chat_id, user_id)
                .until_date(until)
                .await?;
            let dur = format_duration(duration_secs);
            Ok(format!("⏳ {} telah di-ban {}.", html_escape(user_name), dur))
        }
        WarnMode::TMute => {
            let perms = ChatPermissions::empty();
            let until = chrono::Utc::now() + chrono::Duration::seconds(duration_secs as i64);
            bot.restrict_chat_member(chat_id, user_id, perms)
                .until_date(until)
                .await?;
            let dur = format_duration(duration_secs);
            Ok(format!("🔇 {} telah di-mute {}.", html_escape(user_name), dur))
        }
    }
}

/// Handle /warns command - view warnings.
pub async fn warns_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Get target (reply, arg, or self)
    let (target_id, target_name) = if let Some(target) = get_target_from_msg(&bot, &msg, &state).await {
        (target.0, target.1)
    } else if let Some(user) = &msg.from {
        // Fallback to self
        (user.id, user.first_name.clone())
    } else {
        return Ok(());
    };

    let data = state.warns.get_or_create(chat_id.0).await?;

    let user_warns = data.get_user(target_id.0);
    // Note: get_user returns Option<&UserWarns>, we need to access it
    let count = user_warns.map(|uw| uw.active_count(data.config.warn_time_secs)).unwrap_or(0);

    let message = if count == 0 {
        format!(
            "✅ <a href=\"tg://user?id={}\">{}</a> tidak memiliki peringatan.",
            target_id, html_escape(&target_name)
        )
    } else {
        let mut text = format!(
            "⚠️ <a href=\"tg://user?id={}\">{}</a> memiliki {}/{} peringatan:\n\n",
            target_id,
            html_escape(&target_name),
            count,
            data.config.limit
        );

        if let Some(uw) = user_warns {
            for (i, w) in uw.warnings.iter().enumerate() {
                if !w.is_expired(data.config.warn_time_secs) {
                    let reason = w.reason.as_deref().unwrap_or("Tidak ada alasan");
                    text.push_str(&format!("{}. {}\n", i + 1, html_escape(reason)));
                }
            }
        }

        text
    };

    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

/// Handle /rmwarn command - remove latest warning.
pub async fn rmwarn_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let admin_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Permission check
    if !state.permissions.can_restrict_members(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk menghapus peringatan.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Get target
    let (target_id, target_name, _) = match get_target_from_msg(&bot, &msg, &state).await {
        Some(t) => t,
        None => {
            bot.send_message(chat_id, "❌ Siapa yang mau dihapus peringatannya?")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    // Use Repo to remove
    let removed = state.warns.remove_warning(chat_id.0, target_id.0).await?;

    if removed {
        let count = state.warns.get_warning_count(chat_id.0, target_id.0).await?;
        let data = state.warns.get_or_create(chat_id.0).await?;
        
        bot.send_message(
            chat_id,
            format!(
                "✅ Peringatan terakhir untuk <a href=\"tg://user?id={}\">{}</a> telah dihapus.\nPeringatan tersisa: {}/{}",
                target_id,
                html_escape(&target_name),
                count,
                data.config.limit
            ),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    } else {
        bot.send_message(chat_id, "ℹ️ User tidak memiliki peringatan.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    }

    Ok(())
}

/// Handle /resetwarn command - reset user's warnings.
pub async fn resetwarn_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let admin_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Permission check
    if !state.permissions.can_restrict_members(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk menghapus peringatan.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Get target
    let (target_id, target_name, _) = match get_target_from_msg(&bot, &msg, &state).await {
        Some(t) => t,
        None => {
            bot.send_message(chat_id, "❌ Siapa yang mau direset peringatannya?")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    let removed = state.warns.reset_warnings(chat_id.0, target_id.0).await?;

    if removed {
        bot.send_message(
            chat_id,
            format!(
                "✅ Semua peringatan untuk <a href=\"tg://user?id={}\">{}</a> telah dihapus.",
                target_id,
                html_escape(&target_name)
            ),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    } else {
        bot.send_message(chat_id, "ℹ️ User tidak memiliki peringatan.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    }

    Ok(())
}

/// Handle /resetallwarns command - reset ALL warnings.
pub async fn resetallwarns_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let admin_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Requires can_promote_members
    if !state.permissions.can_promote_members(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            "❌ Perintah ini membutuhkan izin 'Tambah Admin' (can_promote_members).",
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    // Manual access needed to clear everything
    let mut data = state.warns.get_or_create(chat_id.0).await?;
    let count = data.user_warns.len();
    data.user_warns.clear();
    state.warns.save(&data).await?;

    bot.send_message(
        chat_id,
        format!("✅ Menghapus peringatan dari <b>{}</b> user.", count),
    )
    .parse_mode(ParseMode::Html)
    .reply_parameters(ReplyParameters::new(msg.id))
    .await?;

    info!("All warnings cleared in chat {} by {}", chat_id, admin_id);
    Ok(())
}

/// Handle /warnings command - view settings.
pub async fn warnings_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    let data = state.warns.get_or_create(chat_id.0).await?;

    let warn_time = match data.config.warn_time_secs {
        Some(secs) => format_duration(secs),
        None => "Permanen (tidak kadaluarsa)".to_string(),
    };

    let message = format!(
        "<b>⚠️ Pengaturan Peringatan</b>\n\n\
        <b>Batas:</b> {}\n\
        <b>Mode:</b> {} ({})\n\
        <b>Durasi hukuman:</b> {}\n\
        <b>Masa berlaku warn:</b> {}",
        data.config.limit,
        data.config.mode.as_str(),
        data.config.mode.description(),
        format_duration(data.config.action_duration_secs),
        warn_time
    );

    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

/// Handle /warnmode command.
pub async fn warnmode_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let admin_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Permission check
    if !state.permissions.can_restrict_members(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk mengubah pengaturan.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    if args.is_empty() {
        // Show current
        let data = state.warns.get_or_create(chat_id.0).await?;
        bot.send_message(
            chat_id,
            format!(
                "Mode peringatan saat ini: <b>{}</b> ({})\n\n\
                Mode tersedia: ban, mute, kick, tban, tmute",
                data.config.mode.as_str(),
                data.config.mode.description()
            ),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    } else {
        match WarnMode::from_str(args[0]) {
            Some(mode) => {
                let mut data = state.warns.get_or_create(chat_id.0).await?;
                data.config.mode = mode.clone();
                state.warns.save(&data).await?;
                bot.send_message(
                    chat_id,
                    format!(
                        "✅ Mode peringatan diubah ke <b>{}</b> ({}).",
                        mode.as_str(),
                        mode.description()
                    ),
                )
                .parse_mode(ParseMode::Html)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            }
            None => {
                bot.send_message(
                    chat_id,
                    "❌ Mode tidak valid. Gunakan: ban, mute, kick, tban, tmute",
                )
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            }
        }
    }

    Ok(())
}

/// Handle /warnlimit command.
pub async fn warnlimit_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let admin_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Permission check
    if !state.permissions.can_restrict_members(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk mengubah pengaturan.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    if args.is_empty() {
        let data = state.warns.get_or_create(chat_id.0).await?;
        bot.send_message(
            chat_id,
            format!("Batas peringatan saat ini: <b>{}</b>", data.config.limit),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    } else if let Ok(limit) = args[0].parse::<u32>() {
        if !(1..=100).contains(&limit) {
            bot.send_message(chat_id, "❌ Batas harus antara 1-100.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        } else {
            let mut data = state.warns.get_or_create(chat_id.0).await?;
            data.config.limit = limit;
            state.warns.save(&data).await?;
            bot.send_message(
                chat_id,
                format!("✅ Batas peringatan diubah ke <b>{}</b>.", limit),
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        }
    } else {
        bot.send_message(chat_id, "❌ Masukkan angka yang valid.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    }

    Ok(())
}

/// Handle /warntime command.
pub async fn warntime_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let admin_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Permission check
    if !state.permissions.can_restrict_members(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk mengubah pengaturan.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    if args.is_empty() {
        let data = state.warns.get_or_create(chat_id.0).await?;
        let warn_time = match data.config.warn_time_secs {
            Some(secs) => format_duration(secs),
            None => "Permanen (tidak kadaluarsa)".to_string(),
        };
        bot.send_message(
            chat_id,
            format!("Masa berlaku peringatan: <b>{}</b>\n\nContoh: /warntime 4w (4 minggu), /warntime off (permanen)", warn_time),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    } else if args[0].to_lowercase() == "off" {
        let mut data = state.warns.get_or_create(chat_id.0).await?;
        data.config.warn_time_secs = None;
        state.warns.save(&data).await?;
        bot.send_message(chat_id, "✅ Peringatan sekarang bersifat permanen (tidak kadaluarsa).")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    } else if let Some(duration) = parse_duration(args[0]) {
        let secs = duration.as_secs();
        let mut data = state.warns.get_or_create(chat_id.0).await?;
        data.config.warn_time_secs = Some(secs);
        state.warns.save(&data).await?;
        bot.send_message(
            chat_id,
            format!("✅ Masa berlaku peringatan diubah ke <b>{}</b>.", format_duration(secs)),
        )
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    } else {
        bot.send_message(
            chat_id,
            "❌ Format tidak valid. Contoh: 4m, 3h, 6d, 5w",
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
    }

    Ok(())
}

/// Handle callback for removing warning.
pub async fn warn_callback_handler(
    bot: ThrottledBot,
    q: CallbackQuery,
    state: AppState,
) -> anyhow::Result<()> {
    let data = match &q.data {
        Some(d) => d,
        None => return Ok(()),
    };

    if !data.starts_with("warn_remove:") {
        return Ok(());
    }

    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() != 3 {
        bot.answer_callback_query(&q.id).text("❌ Data tidak valid.").await?;
        return Ok(());
    }

    let chat_id: i64 = parts[1].parse().unwrap_or(0);
    let target_id: u64 = parts[2].parse().unwrap_or(0);

    if chat_id == 0 || target_id == 0 {
        bot.answer_callback_query(&q.id).text("❌ Data tidak valid.").await?;
        return Ok(());
    }

    let clicker_id = q.from.id;

    // Check if clicker is admin
    if !state.permissions.can_restrict_members(ChatId(chat_id), clicker_id).await.unwrap_or(false) {
        bot.answer_callback_query(&q.id)
            .text("❌ Hanya admin yang dapat menghapus peringatan.")
            .show_alert(true)
            .await?;
        return Ok(());
    }

    // Attempt removal via Repo
    let removed = state.warns.remove_warning(chat_id, target_id).await?;

    if removed {
        let count = state.warns.get_warning_count(chat_id, target_id).await?;
        let data = state.warns.get_or_create(chat_id).await?;

        // Get target name from UserRepo
        let target_name = if let Ok(Some(user)) = state.users.get_by_id(target_id).await {
            user.first_name
        } else {
            format!("User {}", target_id)
        };

        // Update the message with proper mentions
        if let Some(msg) = &q.message {
            let admin_mention = format!(
                "<a href=\"tg://user?id={}\">{}</a>",
                q.from.id,
                html_escape(&q.from.first_name)
            );
            let target_mention = format!(
                "<a href=\"tg://user?id={}\">{}</a>",
                target_id,
                html_escape(&target_name)
            );
            let new_text = format!(
                "✅ Admin {} telah menghapus peringatan {}.\nPeringatan tersisa: {}/{}",
                admin_mention,
                target_mention,
                count,
                data.config.limit
            );
            let _ = bot.edit_message_text(msg.chat().id, msg.id(), new_text)
                .parse_mode(ParseMode::Html)
                .await;
        }

        bot.answer_callback_query(&q.id)
            .text("✅ Peringatan berhasil dihapus.")
            .await?;
    } else {
        bot.answer_callback_query(&q.id)
            .text("ℹ️ User tidak memiliki peringatan.")
            .await?;
    }

    Ok(())
}

/// Get target user from message (reply or args).
/// Returns (user_id, first_name, skip_words_count)
async fn get_target_from_msg(
    bot: &ThrottledBot,
    msg: &Message,
    state: &AppState,
) -> Option<(UserId, String, usize)> {
    // 1. Check reply
    if let Some(reply) = msg.reply_to_message()
        && let Some(user) = &reply.from {
            return Some((user.id, user.first_name.clone(), 0));
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
                    if let MessageEntityKind::TextMention { user } = &entity.kind
                        && entity.offset < 20 {
                            return Some((user.id, user.first_name.clone(), 1));
                        }
                }
            }

            // 4. Try @username via UserRepo
            if arg.starts_with('@') {
                let username = arg.trim_start_matches('@');
                if let Ok(Some(user)) = state.users.get_by_username(username).await {
                    return Some((UserId(user.user_id), user.first_name, 1));
                }
                // Fallback to get_chat (for bots/channels)
                if let Ok(chat) = bot.get_chat(arg.to_string()).await
                    && chat.is_private() {
                        let name = chat.first_name().unwrap_or("User").to_string();
                        return Some((UserId(chat.id.0 as u64), name, 1));
                    }
            }
        }
    }

    None
}

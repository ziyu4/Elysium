//! Approval command handlers.
//!
//! Commands for managing approved users who bypass antiflood.

use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};


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
        bot.send_message(chat_id, "⚠️ Perintah ini hanya untuk grup.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Check if user is admin
    if !state.permissions.is_admin(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Perintah ini hanya untuk admin.")
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
                "📖 <b>Penggunaan:</b>\nReply ke pesan user dengan /approve\nAtau: /approve [user_id]",
            )
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
            return Ok(());
        }
    };

    let chat_title = msg.chat.title().unwrap_or("grup ini");

    if state.message_context.approve_user(chat_id.0, target_id).await? {
        let message = format!(
            "✅ {} telah disetujui di <b>{}</b>!\n\n\
            Mereka sekarang akan diabaikan oleh tindakan otomatis seperti antiflood dan antispam.",
            target_name, html_escape(chat_title)
        );
        bot.send_message(chat_id, message)
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        info!("User {} approved in chat {} by {}", target_id, chat_id, admin_id);
    } else {
        bot.send_message(
            chat_id, 
            format!("ℹ️ {} sudah ada dalam daftar yang disetujui.", target_name)
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

    if !state.permissions.is_admin(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Perintah ini hanya untuk admin.")
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
                "📖 Reply ke pesan user dengan /unapprove\nAtau: /unapprove [user_id]",
            )
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
            return Ok(());
        }
    };

    if state.message_context.unapprove_user(chat_id.0, target_id).await? {
        let message = format!(
            "✅ {} telah dihapus dari daftar persetujuan.\n\n\
            Mereka sekarang tidak lagi bypass antiflood/antispam.",
            target_name
        );
        bot.send_message(chat_id, message)
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        info!("User {} unapproved in chat {} by {}", target_id, chat_id, admin_id);
    } else {
        bot.send_message(
            chat_id, 
            format!("ℹ️ {} tidak ada dalam daftar persetujuan.", target_name)
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

    // Requires can_promote_members (higher level admin)
    if !state.permissions.can_promote_members(chat_id, admin_id).await.unwrap_or(false) {
        bot.send_message(
            chat_id,
            "❌ Perintah ini membutuhkan izin 'Tambah Admin' (can_promote_members).",
        )
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;
        return Ok(());
    }

    let count = state.message_context.unapprove_all(chat_id.0).await?;

    bot.send_message(
        chat_id,
        format!("✅ Menghapus <b>{}</b> user dari daftar persetujuan.", count),
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

    let ctx = state.message_context.get_or_default(chat_id.0).await?;

    let status = if ctx.is_approved(user.id.0) {
        "✅ Anda sudah <b>disetujui</b> di grup ini.\n\nAnda akan mengabaikan tindakan otomatis seperti antiflood dan antispam."
    } else {
        "❌ Anda <b>belum disetujui</b> di grup ini."
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

    let ctx = state.message_context.get_or_default(chat_id.0).await?;

    if ctx.approved_users.is_empty() {
        bot.send_message(chat_id, "📋 Belum ada user yang disetujui di grup ini.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Build list of approved users
    let mut list = String::from("<b>📋 Daftar User yang Disetujui</b>\n\n");
    for (i, user_id) in ctx.approved_users.iter().enumerate() {
        list.push_str(&format!(
            "{}. <a href=\"tg://user?id={}\">{}</a>\n",
            i + 1,
            user_id,
            user_id
        ));
    }
    list.push_str(&format!("\n<i>Total: {} user</i>", ctx.approved_users.len()));

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

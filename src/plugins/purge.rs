//! Purge command handlers.
//!
//! Commands for deleting multiple messages at once.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use teloxide::prelude::*;
use teloxide::types::{MessageId, ReplyParameters, UserId};

use crate::bot::dispatcher::{AppState, ThrottledBot};

/// Global cache for purgefrom markers: chat_id -> message_id
static PURGE_MARKERS: LazyLock<Mutex<HashMap<i64, MessageId>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Handle /purge command - delete messages from reply to now.
/// 
/// Usage:
/// - /purge - delete from replied message to current
/// - /purge <N> - delete N messages after replied message
pub async fn purge_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    purge_action(bot, msg, state, false).await
}

/// Handle /spurge command - silent purge (no confirmation).
pub async fn spurge_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    purge_action(bot, msg, state, true).await
}

async fn purge_action(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    silent: bool,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Check permission: can_delete_messages
    if !state.permissions.can_delete_messages(chat_id, user_id).await.unwrap_or(false) {
        if !silent {
            bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk menghapus pesan.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
        return Ok(());
    }

    // Must be a reply
    let reply = match msg.reply_to_message() {
        Some(r) => r,
        None => {
            if !silent {
                bot.send_message(chat_id, "❌ Reply ke pesan untuk memulai purge.")
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
            return Ok(());
        }
    };

    let start_id = reply.id.0;
    let end_id = msg.id.0;

    // Parse optional count argument
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().collect();
    
    let count = if parts.len() > 1 {
        parts[1].parse::<i32>().ok()
    } else {
        None
    };

    // Collect message IDs to delete
    let mut to_delete: Vec<MessageId> = Vec::new();
    
    if let Some(n) = count {
        // Delete N messages after the replied message
        for i in 0..=n {
            to_delete.push(MessageId(start_id + i));
        }
        // Also delete the command message
        to_delete.push(msg.id);
    } else {
        // Delete all from start to end (inclusive)
        for id in start_id..=end_id {
            to_delete.push(MessageId(id));
        }
    }

    // Delete messages in batches (Telegram allows max 100 per call)
    let deleted_count = delete_messages_batch(&bot, chat_id, &to_delete).await;

    if !silent && deleted_count > 0 {
        let confirm = bot.send_message(
            chat_id, 
            format!("✅ Berhasil menghapus {} pesan.", deleted_count)
        ).await?;
        
        // Auto-delete confirmation after 3 seconds
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let _ = bot.delete_message(chat_id, confirm.id).await;
        });
    }

    Ok(())
}

/// Handle /del command - delete the replied message.
pub async fn del_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Check permission
    if !state.permissions.can_delete_messages(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk menghapus pesan.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Must be a reply
    let reply = match msg.reply_to_message() {
        Some(r) => r,
        None => {
            bot.send_message(chat_id, "❌ Reply ke pesan yang ingin dihapus.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    // Delete both the replied message and the command
    let _ = bot.delete_message(chat_id, reply.id).await;
    let _ = bot.delete_message(chat_id, msg.id).await;

    Ok(())
}

/// Handle /purgefrom command - mark starting point for range purge.
pub async fn purgefrom_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Check permission
    if !state.permissions.can_delete_messages(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk menghapus pesan.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Must be a reply
    let reply = match msg.reply_to_message() {
        Some(r) => r,
        None => {
            bot.send_message(chat_id, "❌ Reply ke pesan untuk menandai titik awal purge.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    // Store the marker
    if let Ok(mut markers) = PURGE_MARKERS.lock() {
        markers.insert(chat_id.0, reply.id);
    }

    // Delete command message
    let _ = bot.delete_message(chat_id, msg.id).await;

    let confirm = bot.send_message(
        chat_id,
        "📍 Titik awal purge ditandai. Gunakan /purgeto untuk menghapus range."
    ).await?;

    // Auto-delete confirmation
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let _ = bot.delete_message(chat_id, confirm.id).await;
    });

    Ok(())
}

/// Handle /purgeto command - delete from marked purgefrom to this reply.
pub async fn purgeto_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0));

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Check permission
    if !state.permissions.can_delete_messages(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda tidak memiliki izin untuk menghapus pesan.")
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    // Must be a reply
    let reply = match msg.reply_to_message() {
        Some(r) => r,
        None => {
            bot.send_message(chat_id, "❌ Reply ke pesan untuk menandai titik akhir purge.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    // Get the purgefrom marker
    let start_id_result: Result<Option<i32>, ()> = {
        match PURGE_MARKERS.lock() {
            Ok(mut markers) => Ok(markers.remove(&chat_id.0).map(|id| id.0)),
            Err(_) => Err(()),
        }
    };
    
    let start_id = match start_id_result {
        Ok(Some(id)) => id,
        Ok(None) => {
            bot.send_message(chat_id, "❌ Tidak ada titik awal. Gunakan /purgefrom dulu.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
        Err(_) => {
            bot.send_message(chat_id, "❌ Internal error.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            return Ok(());
        }
    };

    let end_id = reply.id.0;

    // Ensure start <= end
    let (actual_start, actual_end) = if start_id <= end_id {
        (start_id, end_id)
    } else {
        (end_id, start_id)
    };

    // Collect message IDs
    let mut to_delete: Vec<MessageId> = Vec::new();
    for id in actual_start..=actual_end {
        to_delete.push(MessageId(id));
    }
    // Also delete command message
    to_delete.push(msg.id);

    let deleted_count = delete_messages_batch(&bot, chat_id, &to_delete).await;

    if deleted_count > 0 {
        let confirm = bot.send_message(
            chat_id,
            format!("✅ Berhasil menghapus {} pesan.", deleted_count)
        ).await?;

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let _ = bot.delete_message(chat_id, confirm.id).await;
        });
    }

    Ok(())
}

/// Delete messages in batches (Telegram limit is typically handled server-side,
/// but we process one by one for reliability).
async fn delete_messages_batch(
    bot: &ThrottledBot,
    chat_id: teloxide::types::ChatId,
    message_ids: &[MessageId],
) -> usize {
    let mut deleted = 0;
    
    for &msg_id in message_ids {
        if bot.delete_message(chat_id, msg_id).await.is_ok() {
            deleted += 1;
        }
    }
    
    deleted
}

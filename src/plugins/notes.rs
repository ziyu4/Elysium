//! Notes command handlers.
//!
//! Commands for saving and retrieving notes.

use teloxide::prelude::*;
use teloxide::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, InputFile, MessageId, ParseMode, ReplyParameters,
};
use tracing::info;

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::{GroupSettingsRepo, InlineButton, Note};
use crate::utils::{apply_fillings, apply_rules_filling, html_escape, parse_note_content, ReplyExt};

/// Handle /save command - save a note.
pub async fn save_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(u) => u.id,
        None => return Ok(()),
    };

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(chat_id, "⚠️ Perintah ini hanya untuk grup.")
            .await?;
        return Ok(());
    }

    // Check permission
    if !state
        .permissions
        .can_change_info(chat_id, user_id)
        .await
        .unwrap_or(false)
    {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .await?;
        return Ok(());
    }

    // Parse command: /save <name> [content]
    let text = msg.text().or_else(|| msg.caption()).unwrap_or("");
    let parts: Vec<&str> = text.splitn(3, char::is_whitespace).collect();

    if parts.len() < 2 {
        bot.send_message(
            chat_id,
            "<b>📖 Cara menyimpan note:</b>\n\n\
            <code>/save nama_note Isi note</code>\n\
            atau reply ke pesan dengan:\n\
            <code>/save nama_note</code>\n\n\
            <b>Contoh:</b>\n\
            <code>/save rules Baca peraturan!</code>\n\
            <code>/save info [Klik](url) {admin}</code>",
        )
        .parse_mode(ParseMode::Html)
        .await?;
        return Ok(());
    }

    let note_name = parts[1].to_lowercase();
    
    // Validate note name
    if note_name.is_empty() || note_name.len() > 50 {
        bot.send_message(chat_id, "❌ Nama note harus 1-50 karakter.")
            .await?;
        return Ok(());
    }

    // Get content from args or reply
    let (content, media_file_id, media_type) = if let Some(reply) = msg.reply_to_message() {
        let text = reply.text().or_else(|| reply.caption()).map(String::from);
        let (file_id, mtype) = extract_media(reply);
        (text, file_id, mtype)
    } else {
        let text = parts.get(2).map(|s| s.to_string());
        (text, None, None)
    };

    if content.is_none() && media_file_id.is_none() {
        bot.send_message(chat_id, "❌ Note harus memiliki konten atau media.")
            .await?;
        return Ok(());
    }

    // Parse the content for buttons and tags
    let parsed = content
        .as_ref()
        .map(|c| parse_note_content(c))
        .unwrap_or_else(|| crate::utils::ParsedNote {
            text: String::new(),
            buttons: vec![],
            tags: Default::default(),
        });

    // Create note
    let mut note = Note::new(&note_name);
    note.text = if parsed.text.is_empty() {
        None
    } else {
        Some(parsed.text)
    };
    note.media_file_id = media_file_id;
    note.media_type = media_type;
    note.buttons = parsed.buttons;
    note.tags = parsed.tags;

    // Save to database
    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;
    settings.notes.save(note);
    repo.save(&settings).await?;

    bot.send_message(
        chat_id,
        format!("✅ Note <code>{}</code> berhasil disimpan!\nGunakan <code>/get {}</code> atau <code>#{}</code>", 
            html_escape(&note_name), note_name, note_name),
    )
    .reply_parameters(ReplyParameters::new(msg.id))
    .parse_mode(ParseMode::Html)
    .await?;

    info!("Note '{}' saved in chat {}", note_name, chat_id);
    Ok(())
}

/// Handle /get command - retrieve a note.
pub async fn get_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    // Parse note name
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().collect();

    if parts.len() < 2 {
        bot.send_message(chat_id, "📖 Gunakan: /get <nama_note>")
            .await?;
        return Ok(());
    }

    let note_name = parts[1].to_lowercase();
    let noformat = parts.get(2).map(|s| s.to_lowercase()) == Some("noformat".to_string());

    send_note(&bot, &state, chat_id, user, &note_name, &state.bot_username, noformat, false, msg.reply_target()).await
}

/// Handle #notename shortcut.
pub async fn handle_hashtag_note(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let text = msg.text().unwrap_or("");
    
    // Check if starts with #
    if !text.starts_with('#') {
        return Ok(());
    }

    let note_name = text
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_start_matches('#')
        .to_lowercase();

    if note_name.is_empty() {
        return Ok(());
    }

    let chat_id = msg.chat.id;
    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    // Smart reply: if user replied to someone, note replies to that person
    send_note(&bot, &state, chat_id, user, &note_name, &state.bot_username, false, true, msg.note_reply_target()).await
}

/// Handle /notes or /saved command - list all notes.
pub async fn notes_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;

    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    // Get note
    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let settings = repo.get_or_create(chat_id.0).await?;

    let names = settings.notes.list_names();
    if names.is_empty() {
        bot.send_message(chat_id, "📋 Belum ada notes di grup ini.")
            .await?;
        return Ok(());
    }

    let mut list = String::from("<b>📋 Daftar Notes</b>\n\n");
    for name in names {
        list.push_str(&format!("• <code>{}</code>\n", html_escape(name)));
    }
    list.push_str(&format!("\n<i>Total: {} notes</i>", settings.notes.notes.len()));

    bot.send_message(chat_id, list)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

/// Handle /clear command - delete a note.
pub async fn clear_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(u) => u.id,
        None => return Ok(()),
    };

    if !state.permissions.can_change_info(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().collect();

    if parts.len() < 2 {
        bot.send_message(chat_id, "📖 Gunakan: /clear <nama_note>")
            .await?;
        return Ok(());
    }

    let note_name = parts[1].to_lowercase();

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;

    if settings.notes.delete(&note_name).is_some() {
        repo.save(&settings).await?;
        bot.send_message(chat_id, format!("✅ Note <code>{}</code> dihapus.", html_escape(&note_name)))
            .parse_mode(ParseMode::Html)
            .await?;
        info!("Note '{}' deleted in chat {}", note_name, chat_id);
    } else {
        bot.send_message(chat_id, format!("❌ Note <code>{}</code> tidak ditemukan.", html_escape(&note_name)))
            .parse_mode(ParseMode::Html)
            .await?;
    }

    Ok(())
}

/// Handle /clearall command - delete all notes.
pub async fn clearall_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(u) => u.id,
        None => return Ok(()),
    };

    // Requires higher permission
    if !state.permissions.can_promote_members(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Perintah ini membutuhkan izin 'Tambah Admin'.")
            .await?;
        return Ok(());
    }

    // Clear note
    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;

    let count = settings.notes.clear_all();
    repo.save(&settings).await?;

    bot.send_message(chat_id, format!("✅ Menghapus {} notes.", count))
        .await?;

    info!("All {} notes deleted in chat {} by {}", count, chat_id, user_id);
    Ok(())
}

/// Handle /privatenotes command - toggle private notes.
pub async fn privatenotes_command(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let user_id = match msg.from.as_ref() {
        Some(u) => u.id,
        None => return Ok(()),
    };

    if !state.permissions.can_change_info(chat_id, user_id).await.unwrap_or(false) {
        bot.send_message(chat_id, "❌ Anda harus admin dengan izin 'Ubah Info Grup'.")
            .await?;
        return Ok(());
    }

    let text = msg.text().unwrap_or("");
    let args: Vec<&str> = text.split_whitespace().skip(1).collect();

    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let mut settings = repo.get_or_create(chat_id.0).await?;

    if args.is_empty() {
        // Toggle
        settings.notes.private_notes = !settings.notes.private_notes;
    } else {
        match args[0].to_lowercase().as_str() {
            "on" | "yes" | "true" => settings.notes.private_notes = true,
            "off" | "no" | "false" => settings.notes.private_notes = false,
            _ => {
                bot.send_message(chat_id, "📖 Gunakan: /privatenotes on/off")
                    .await?;
                return Ok(());
            }
        }
    }

    repo.save(&settings).await?;

    let status = if settings.notes.private_notes {
        "✅ Notes akan dikirim ke PM."
    } else {
        "✅ Notes akan dikirim ke grup."
    };

    bot.send_message(chat_id, status).await?;
    Ok(())
}

/// Send a note to the user.
async fn send_note(
    bot: &ThrottledBot,
    state: &AppState,
    chat_id: ChatId,
    user: &teloxide::types::User,
    note_name: &str,
    bot_username: &str,
    noformat: bool,
    silent_if_not_found: bool,
    reply_to: MessageId,
) -> anyhow::Result<()> {
    let repo = GroupSettingsRepo::new(&state.db, &state.cache);
    let settings = repo.get_or_create(chat_id.0).await?;

    let note = match settings.notes.get(note_name) {
        Some(n) => n,
        None => {
            if !silent_if_not_found {
                bot.send_message(chat_id, format!("❌ Note <code>{}</code> tidak ditemukan.", html_escape(note_name)))
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
            return Ok(());
        }
    };

    // Check admin-only
    if note.tags.admin_only {
        if !state.permissions.is_admin(chat_id, user.id).await.unwrap_or(false) {
            bot.send_message(chat_id, "❌ Note ini hanya untuk admin.")
                .await?;
            return Ok(());
        }
    }

    // Determine where to send
    let send_to_pm = if note.tags.is_private {
        true
    } else if note.tags.no_private {
        false
    } else {
        settings.notes.private_notes
    };

    let target_chat = if send_to_pm {
        ChatId(user.id.0 as i64)
    } else {
        chat_id
    };

    // Build message
    let chat_name = settings.title.as_deref().unwrap_or("Grup");
    let text = if noformat {
        note.text.clone().unwrap_or_default()
    } else {
        let filled = apply_fillings(
            note.text.as_deref().unwrap_or(""),
            user,
            chat_name,
            bot_username,
        );
        let (filled, _rules_buttons) = apply_rules_filling(&filled, chat_id.0, bot_username);
        filled
    };

    // Build keyboard
    let keyboard = if noformat || note.buttons.is_empty() {
        InlineKeyboardMarkup::default()
    } else {
        build_keyboard(&note.buttons)
    };

    // Send message with reply (only in group, not PM)
    let has_keyboard = !note.buttons.is_empty() && !noformat;
    let should_reply = target_chat == chat_id; // Don't reply when sending to PM

    if let Some(ref file_id) = note.media_file_id {
        match note.media_type.as_deref() {
            Some("photo") => {
                let mut req = bot.send_photo(target_chat, InputFile::file_id(file_id));
                if !text.is_empty() {
                    req = req.caption(text).parse_mode(ParseMode::Html);
                }
                if has_keyboard {
                    req = req.reply_markup(keyboard);
                }
                if note.tags.media_spoiler {
                    req = req.has_spoiler(true);
                }
                if note.tags.protect {
                    req = req.protect_content(true);
                }
                if should_reply {
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                }
                req.await?;
            }
            Some("video") => {
                let mut req = bot.send_video(target_chat, InputFile::file_id(file_id));
                if !text.is_empty() {
                    req = req.caption(text).parse_mode(ParseMode::Html);
                }
                if has_keyboard {
                    req = req.reply_markup(keyboard);
                }
                if note.tags.media_spoiler {
                    req = req.has_spoiler(true);
                }
                if note.tags.protect {
                    req = req.protect_content(true);
                }
                if should_reply {
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                }
                req.await?;
            }
            Some("animation") => {
                let mut req = bot.send_animation(target_chat, InputFile::file_id(file_id));
                if !text.is_empty() {
                    req = req.caption(text).parse_mode(ParseMode::Html);
                }
                if has_keyboard {
                    req = req.reply_markup(keyboard);
                }
                if note.tags.media_spoiler {
                    req = req.has_spoiler(true);
                }
                if note.tags.protect {
                    req = req.protect_content(true);
                }
                if should_reply {
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                }
                req.await?;
            }
            Some("sticker") => {
                bot.send_sticker(target_chat, InputFile::file_id(file_id))
                    .await?;
                if !text.is_empty() {
                    let mut req = bot.send_message(target_chat, text)
                        .parse_mode(ParseMode::Html);
                    if has_keyboard {
                        req = req.reply_markup(keyboard);
                    }
                    if should_reply {
                        req = req.reply_parameters(ReplyParameters::new(reply_to));
                    }
                    req.await?;
                }
            }
            Some("document") => {
                let mut req = bot.send_document(target_chat, InputFile::file_id(file_id));
                if !text.is_empty() {
                    req = req.caption(text).parse_mode(ParseMode::Html);
                }
                if has_keyboard {
                    req = req.reply_markup(keyboard);
                }
                if note.tags.protect {
                    req = req.protect_content(true);
                }
                if should_reply {
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                }
                req.await?;
            }
            _ => {
                // Unknown, send as text
                let mut req = bot.send_message(target_chat, text)
                    .parse_mode(ParseMode::Html);
                if has_keyboard {
                    req = req.reply_markup(keyboard);
                }
                if note.tags.protect {
                    req = req.protect_content(true);
                }
                if should_reply {
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                }
                req.await?;
            }
        }
    } else if !text.is_empty() {
        let mut req = bot.send_message(target_chat, &text)
            .parse_mode(ParseMode::Html);
        if has_keyboard {
            req = req.reply_markup(keyboard);
        }
        if note.tags.protect {
            req = req.protect_content(true);
        }
        if should_reply {
            req = req.reply_parameters(ReplyParameters::new(reply_to));
        }
        req.await?;
    }

    // If sent to PM, send confirmation in group
    if send_to_pm {
        bot.send_message(chat_id, "📨 Note dikirim ke PM Anda.")
            .await?;
    }

    Ok(())
}

/// Build inline keyboard from buttons.
fn build_keyboard(buttons: &[Vec<InlineButton>]) -> InlineKeyboardMarkup {
    let keyboard: Vec<Vec<InlineKeyboardButton>> = buttons
        .iter()
        .map(|row| {
            row.iter()
                .filter_map(|btn| {
                    btn.url.parse().ok().map(|url| {
                        InlineKeyboardButton::url(&btn.text, url)
                    })
                })
                .collect()
        })
        .filter(|row: &Vec<InlineKeyboardButton>| !row.is_empty())
        .collect();

    InlineKeyboardMarkup::new(keyboard)
}

/// Extract media from message.
fn extract_media(msg: &Message) -> (Option<String>, Option<String>) {
    if let Some(photo) = msg.photo() {
        let largest = photo.iter().max_by_key(|p| p.width * p.height);
        return (largest.map(|p| p.file.id.clone()), Some("photo".to_string()));
    }
    if let Some(video) = msg.video() {
        return (Some(video.file.id.clone()), Some("video".to_string()));
    }
    if let Some(animation) = msg.animation() {
        return (Some(animation.file.id.clone()), Some("animation".to_string()));
    }
    if let Some(sticker) = msg.sticker() {
        return (Some(sticker.file.id.clone()), Some("sticker".to_string()));
    }
    if let Some(document) = msg.document() {
        return (Some(document.file.id.clone()), Some("document".to_string()));
    }
    (None, None)
}

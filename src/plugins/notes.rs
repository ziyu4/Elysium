//! Notes plugin.
//! 
//! Handles saving and retrieving notes using decentralized repository.

use teloxide::prelude::*;
use teloxide::types::{ParseMode, ReplyParameters, InlineKeyboardMarkup, InlineKeyboardButton, InputFile};

use crate::bot::dispatcher::{AppState, ThrottledBot};
use crate::database::models::DbNote;
use crate::utils::{apply_fillings_new, html_escape, parser::parse_buttons};
use crate::i18n::get_text;

async fn save_note(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    args: &[&str],
) -> anyhow::Result<()> {
    let locale = state.get_locale(Some(msg.chat.id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;

    // Must have at least the note name
    if args.is_empty() {
        bot.send_message(msg.chat.id, get_text(&locale, "notes.save_usage"))
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let name = args[0].to_lowercase();
    
    // Determine content and media from multiple sources:
    // 1. Inline content after name: /save name content here
    // 2. Reply message text/caption
    // 3. Reply message media
    
    let full_text = msg.text().unwrap_or("");
    let inline_content = full_text
        .split_once(&name)
        .map(|(_, rest)| rest.trim())
        .unwrap_or("");
    
    let reply = msg.reply_to_message();
    
    // Get content: prefer inline, then reply text/caption
    let content = if !inline_content.is_empty() {
        inline_content.to_string()
    } else if let Some(reply_msg) = reply {
        // Try text first, then caption
        reply_msg.text()
            .or_else(|| reply_msg.caption())
            .map(String::from)
            .unwrap_or_default()
    } else {
        String::new()
    };
    
    // Get media from reply
    let (file_id, file_type) = if let Some(reply_msg) = reply {
        if let Some(photo) = reply_msg.photo().and_then(|p| p.last()) {
            (Some(photo.file.id.clone()), Some("photo".to_string()))
        } else if let Some(video) = reply_msg.video() {
            (Some(video.file.id.clone()), Some("video".to_string()))
        } else if let Some(doc) = reply_msg.document() {
            (Some(doc.file.id.clone()), Some("document".to_string()))
        } else if let Some(anim) = reply_msg.animation() {
            (Some(anim.file.id.clone()), Some("animation".to_string()))
        } else if let Some(sticker) = reply_msg.sticker() {
            (Some(sticker.file.id.clone()), Some("sticker".to_string()))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };
    
    // Must have either content or media
    if content.is_empty() && file_id.is_none() {
        bot.send_message(msg.chat.id, get_text(&locale, "notes.error_empty_content"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }
    
    // Parse buttons if any
    let (clean_content, buttons) = parse_buttons(&content);
    
    let mut note = DbNote::new(msg.chat.id.0, &name, &clean_content);
    note.buttons = buttons;
    note.file_id = file_id;
    note.file_type = file_type;

    state.notes.save_note(&note).await?;

    bot.send_message(msg.chat.id, get_text(&locale, "notes.saved").replace("{name}", &html_escape(&name)))
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

async fn list_notes(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    // Uses L1 Cache (Keys Only)
    let names = state.notes.get_names(msg.chat.id.0).await?;

    let locale = state.get_locale(Some(msg.chat.id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;

    if names.is_empty() {
        bot.send_message(msg.chat.id, get_text(&locale, "notes.list_empty"))
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
        return Ok(());
    }

    let notes_list = names
        .iter()
        .map(|n| format!("• <code>{}</code>", html_escape(n)))
        .collect::<Vec<_>>()
        .join("\n");

    let notes_footer = get_text(&locale, "notes.list_footer");

    bot.send_message(msg.chat.id, format!("{}\n\n{}\n\n{}", get_text(&locale, "notes.list_header"), notes_list, notes_footer))
        .parse_mode(ParseMode::Html)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    Ok(())
}

async fn get_note(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    args: &[&str],
) -> anyhow::Result<()> {
    if args.is_empty() {
        return Ok(());
    }
    let name = args[0].to_lowercase();
    let name_clean = name.trim_start_matches('#');
    
    // Uses L2 Cache (Content)
    if let Some(note) = state.notes.get_note(msg.chat.id.0, name_clean).await? {
        send_note_response(&bot, &msg, &note).await?;
    } else {
        let locale = state.get_locale(Some(msg.chat.id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;
        bot.send_message(msg.chat.id, get_text(&locale, "notes.not_found").replace("{name}", &html_escape(name_clean)))
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    }
    Ok(())
}

async fn clear_note(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    args: &[&str],
) -> anyhow::Result<()> {
    if args.is_empty() {
        return Ok(());
    }
    let name = args[0].to_lowercase();
    
    // Resolve locale
    let locale = state.get_locale(Some(msg.chat.id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;

    if state.notes.delete_note(msg.chat.id.0, &name).await? {
        bot.send_message(msg.chat.id, get_text(&locale, "notes.deleted").replace("{name}", &html_escape(&name)))
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    } else {
        bot.send_message(msg.chat.id, get_text(&locale, "notes.not_found").replace("{name}", &html_escape(&name)))
            .parse_mode(ParseMode::Html)
            .reply_parameters(ReplyParameters::new(msg.id))
            .await?;
    }
    Ok(())
}

async fn send_note_response(
    bot: &ThrottledBot,
    msg: &Message,
    note: &DbNote,
) -> anyhow::Result<()> {
    let user = msg.from.as_ref().unwrap();
     // Apply fillings if needed
    let text = apply_fillings_new(&note.content, user, "Grup", None);

    // Build keyboard
    let keyboard = if !note.buttons.is_empty() {
        let rows: Vec<Vec<InlineKeyboardButton>> = note
            .buttons
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
            .filter(|row: &Vec<_>| !row.is_empty())
            .collect();
        
        if rows.is_empty() {
            None
        } else {
            Some(InlineKeyboardMarkup::new(rows))
        }
    } else {
        None
    };

    let chat_id = msg.chat.id;
    let reply_to = msg.reply_to_message().map(|m| m.id).unwrap_or(msg.id);

    // Send based on media
     match (&note.file_id, &note.file_type) {
        (Some(file_id), Some(media_type)) => {
             match media_type.as_str() {
                "photo" => {
                    let mut req = bot.send_photo(chat_id, InputFile::file_id(file_id));
                    if !text.is_empty() { req = req.caption(&text).parse_mode(ParseMode::Html); }
                    if let Some(kb) = keyboard { req = req.reply_markup(kb); }
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                    req.await?;
                }
                "video" => {
                    let mut req = bot.send_video(chat_id, InputFile::file_id(file_id));
                    if !text.is_empty() { req = req.caption(&text).parse_mode(ParseMode::Html); }
                    if let Some(kb) = keyboard { req = req.reply_markup(kb); }
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                    req.await?;
                }
                "document" => {
                    let mut req = bot.send_document(chat_id, InputFile::file_id(file_id));
                    if !text.is_empty() { req = req.caption(&text).parse_mode(ParseMode::Html); }
                    if let Some(kb) = keyboard { req = req.reply_markup(kb); }
                    req = req.reply_parameters(ReplyParameters::new(reply_to));
                    req.await?;
                }
                _ => {}
            }
        }
        _ => {
            let mut req = bot.send_message(chat_id, &text).parse_mode(ParseMode::Html);
            if let Some(kb) = keyboard { req = req.reply_markup(kb); }
            req = req.reply_parameters(ReplyParameters::new(reply_to));
            req.await?;
        }
    }
    
    Ok(())
}

// ============================================================================
// Public wrapper functions for mod.rs
// ============================================================================

/// Save note command.
pub async fn save_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }
    let text = msg.text().unwrap_or("").to_string();
    let parts: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
    let args: Vec<&str> = parts.iter().skip(1).map(|s| s.as_str()).collect();
    save_note(bot, msg, state, &args).await
}

/// List notes command.
pub async fn notes_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }
    list_notes(bot, msg, state).await
}

/// Clear note command.
pub async fn clear_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }
    let text = msg.text().unwrap_or("").to_string();
    let parts: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
    let args: Vec<&str> = parts.iter().skip(1).map(|s| s.as_str()).collect();
    clear_note(bot, msg, state, &args).await
}

/// Clear all notes command (placeholder - requires admin check).
pub async fn clearall_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }
    // TODO: Implement clearall with admin permission check
    let locale = state.get_locale(Some(msg.chat.id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;
    bot.send_message(msg.chat.id, get_text(&locale, "notes.error_clearall_impl"))
        .await?;
    Ok(())
}

/// Toggle private notes command (placeholder).
pub async fn privatenotes_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }
    // TODO: Implement private notes toggle
    let locale = state.get_locale(Some(msg.chat.id.0), Some(msg.from.as_ref().map(|u| u.id.0).unwrap_or(0))).await;
    bot.send_message(msg.chat.id, get_text(&locale, "notes.error_privatenotes_impl"))
        .await?;
    Ok(())
}

/// Get note command.
pub async fn get_command(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }
    let text = msg.text().unwrap_or("").to_string();
    let parts: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
    let args: Vec<&str> = parts.iter().skip(1).map(|s| s.as_str()).collect();
    get_note(bot, msg, state, &args).await
}

/// Handle hashtag note shortcuts (e.g., #notename).
pub async fn handle_hashtag_note(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        return Ok(());
    }

    let text = match msg.text() {
        Some(t) => t.to_string(),
        None => return Ok(()),
    };

    // Extract note name from hashtag
    if !text.starts_with('#') || text.starts_with("##") {
        return Ok(());
    }

    let note_name = text[1..].split_whitespace().next().unwrap_or("").to_lowercase();
    if note_name.is_empty() {
        return Ok(());
    }

    // Get and send note
    if let Some(note) = state.notes.get_note(msg.chat.id.0, &note_name).await? {
        send_note_response(&bot, &msg, &note).await?;
    }

    Ok(())
}



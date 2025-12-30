//! Reply helper utilities.
//!
//! Provides consistent reply behavior across all handlers.

use teloxide::types::{Message, MessageId};

/// Determines what message the bot should reply to.
///
/// For notes with hashtag (#note):
/// - If user replied to someone's message, reply to that message
/// - Otherwise, reply to the user's message
///
/// For commands:
/// - Always reply to the command message
pub fn get_reply_target(msg: &Message, is_note_hashtag: bool) -> MessageId {
    if is_note_hashtag {
        // For notes: reply to what user replied to (if any)
        if let Some(reply) = msg.reply_to_message() {
            return reply.id;
        }
    }
    // Default: reply to the original message
    msg.id
}

/// Extension trait for easier reply handling.
pub trait ReplyExt {
    /// Get the message ID to reply to for command responses.
    fn reply_target(&self) -> MessageId;
    
    /// Get the message ID to reply to for notes (smart reply).
    fn note_reply_target(&self) -> MessageId;
}

impl ReplyExt for Message {
    fn reply_target(&self) -> MessageId {
        get_reply_target(self, false)
    }
    
    fn note_reply_target(&self) -> MessageId {
        get_reply_target(self, true)
    }
}

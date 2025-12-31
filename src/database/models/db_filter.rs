//! Filter model for distinct collection.

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::database::InlineButton;

/// How to match the trigger.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    /// Match anywhere in message (default)
    #[default]
    Keyword,
    /// Match only if entire message equals trigger
    Exact,
    /// Match if message starts with trigger
    Prefix,
}

/// A single filter document (stored in `filters` collection).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbFilter {
    /// MongoDB document ID
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Chat ID this filter belongs to
    pub chat_id: i64,

    /// Trigger word/phrase (indexed)
    pub trigger: String,

    /// How to match the trigger
    #[serde(default)]
    pub match_type: MatchType,

    /// Reply text
    pub reply: String,

    /// Buttons for the reply
    #[serde(default)]
    pub buttons: Vec<Vec<InlineButton>>,

    /// Media file ID (if any)
    #[serde(default)]
    pub media_file_id: Option<String>,

    /// Media type (photo, video, animation, document, sticker)
    #[serde(default)]
    pub media_type: Option<String>,

    /// Only admins can trigger this filter
    #[serde(default)]
    pub admin_only: bool,

    /// Only non-admins can trigger this filter
    #[serde(default)]
    pub user_only: bool,

    /// Protect content (can't be forwarded)
    #[serde(default)]
    pub protect: bool,

    /// Reply to the user being replied to
    #[serde(default)]
    pub replytag: bool,
}

impl DbFilter {
    /// Check if a message matches this filter's trigger.
    pub fn matches(&self, message: &str) -> bool {
        let msg_lower = message.to_lowercase();
        let trigger_lower = self.trigger.to_lowercase();

        match self.match_type {
            MatchType::Keyword => msg_lower.contains(&trigger_lower),
            MatchType::Exact => msg_lower.trim() == trigger_lower,
            MatchType::Prefix => msg_lower.starts_with(&trigger_lower),
        }
    }
}

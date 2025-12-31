//! Bye/Goodbye configuration model for separate collection.

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use super::common::InlineButton;

/// Goodbye configuration stored in its own collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByeSettings {
    /// MongoDB document ID
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Telegram chat ID (indexed)
    pub chat_id: i64,

    /// Whether goodbye messages are enabled
    #[serde(default)]
    pub enabled: bool,

    /// Goodbye message template
    #[serde(default)]
    pub message: Option<String>,

    /// Media file ID (if any)
    #[serde(default)]
    pub media_file_id: Option<String>,

    /// Media type (photo, video, animation, sticker)
    #[serde(default)]
    pub media_type: Option<String>,

    /// Inline buttons for the goodbye message
    #[serde(default)]
    pub buttons: Vec<Vec<InlineButton>>,
}

impl Default for ByeSettings {
    fn default() -> Self {
        Self {
            id: None,
            chat_id: 0,
            enabled: false,
            message: None,
            media_file_id: None,
            media_type: None,
            buttons: Vec::new(),
        }
    }
}

impl ByeSettings {
    /// Create new settings for a chat.
    pub fn new(chat_id: i64) -> Self {
        Self {
            chat_id,
            ..Default::default()
        }
    }
}

//! Welcome configuration model for separate collection.

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use super::common::InlineButton;

/// Welcome configuration stored in its own collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeSettings {
    /// MongoDB document ID
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Telegram chat ID (indexed)
    pub chat_id: i64,

    /// Whether welcome messages are enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Welcome message template
    #[serde(default)]
    pub message: Option<String>,

    /// Media file ID (if any)
    #[serde(default)]
    pub media_file_id: Option<String>,

    /// Media type (photo, video, animation, sticker)
    #[serde(default)]
    pub media_type: Option<String>,

    /// Inline buttons for the welcome message
    #[serde(default)]
    pub buttons: Vec<Vec<InlineButton>>,

    /// Delete previous welcome message when new member joins
    #[serde(default)]
    pub clean_welcome: bool,
}

fn default_enabled() -> bool {
    true
}

impl Default for WelcomeSettings {
    fn default() -> Self {
        Self {
            id: None,
            chat_id: 0,
            enabled: true,
            message: None,
            media_file_id: None,
            media_type: None,
            buttons: Vec::new(),
            clean_welcome: false,
        }
    }
}

impl WelcomeSettings {
    /// Create new settings for a chat.
    pub fn new(chat_id: i64) -> Self {
        Self {
            chat_id,
            ..Default::default()
        }
    }
}

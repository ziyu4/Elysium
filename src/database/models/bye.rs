//! Goodbye message configuration models.

use serde::{Deserialize, Serialize};

use super::common::InlineButton;

/// Goodbye message configuration for a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByeConfig {
    /// Whether goodbye is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Goodbye message text (supports formatting placeholders)
    /// Placeholders: {name}, {username}, {mention}, {id}, {group}
    #[serde(default)]
    pub message: Option<String>,

    /// Media file_id (cached in media channel)
    #[serde(default)]
    pub media_file_id: Option<String>,

    /// Media type: "photo", "video", "animation", "sticker"
    #[serde(default)]
    pub media_type: Option<String>,

    /// Inline buttons (rows of buttons)
    #[serde(default)]
    pub buttons: Vec<Vec<InlineButton>>,

    /// Whether to delete the service message
    #[serde(default)]
    pub delete_service_message: bool,
}

impl Default for ByeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            message: Some("👋 Selamat tinggal, {mention}!".to_string()),
            media_file_id: None,
            media_type: None,
            buttons: vec![],
            delete_service_message: false,
        }
    }
}

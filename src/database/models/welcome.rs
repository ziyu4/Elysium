//! Welcome message configuration models.

use serde::{Deserialize, Serialize};

use super::common::InlineButton;

/// Welcome message configuration for a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeConfig {
    /// Whether welcome is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Welcome message text (supports formatting placeholders)
    /// Placeholders: {name}, {username}, {mention}, {id}, {group}, {count}
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

fn default_true() -> bool {
    true
}

impl Default for WelcomeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            message: Some("👋 Selamat datang, {mention}!".to_string()),
            media_file_id: None,
            media_type: None,
            buttons: vec![],
            delete_service_message: false,
        }
    }
}

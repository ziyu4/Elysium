//! Rules configuration model for separate collection.

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

/// Rules configuration stored in its own collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesSettings {
    /// MongoDB document ID
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Telegram chat ID (indexed)
    pub chat_id: i64,

    /// The rules text (supports newlines and formatting)
    #[serde(default)]
    pub text: Option<String>,

    /// Whether to show rules in PM (true) or in group (false)
    #[serde(default)]
    pub show_in_pm: bool,

    /// Button text for "View Rules" button
    #[serde(default = "default_rules_button")]
    pub button_text: String,
}

fn default_rules_button() -> String {
    "📜 Baca Peraturan".to_string()
}

impl Default for RulesSettings {
    fn default() -> Self {
        Self {
            id: None,
            chat_id: 0,
            text: None,
            show_in_pm: false,
            button_text: default_rules_button(),
        }
    }
}

impl RulesSettings {
    /// Create new settings for a chat.
    pub fn new(chat_id: i64) -> Self {
        Self {
            chat_id,
            ..Default::default()
        }
    }

    /// Check if rules are set.
    pub fn has_rules(&self) -> bool {
        self.text.as_ref().is_some_and(|t| !t.is_empty())
    }
}

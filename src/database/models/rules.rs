//! Rules configuration models.

use serde::{Deserialize, Serialize};

/// Rules configuration for a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesConfig {
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

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            text: None,
            show_in_pm: false,
            button_text: default_rules_button(),
        }
    }
}

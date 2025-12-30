//! Common shared models.

use serde::{Deserialize, Serialize};

/// Generic inline button for messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineButton {
    /// Button text
    pub text: String,
    /// URL to open when clicked
    pub url: String,
}

impl InlineButton {
    /// Create a new inline button.
    pub fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
        }
    }
}

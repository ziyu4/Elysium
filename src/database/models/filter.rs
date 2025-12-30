//! Filter model for auto-reply triggers.

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

/// A chat filter for auto-replies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    /// Trigger word/phrase
    pub trigger: String,

    /// How to match the trigger
    #[serde(default)]
    pub match_type: MatchType,

    /// Reply text (with fillings support)
    pub reply: String,

    /// Buttons for the reply
    #[serde(default)]
    pub buttons: Vec<Vec<InlineButton>>,

    /// Media file ID (if any)
    #[serde(default)]
    pub media_file_id: Option<String>,

    /// Media type (photo, video, animation, document)
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

impl Filter {
    /// Create a new text filter.
    pub fn new(trigger: impl Into<String>, reply: impl Into<String>) -> Self {
        Self {
            trigger: trigger.into().to_lowercase(),
            match_type: MatchType::default(),
            reply: reply.into(),
            buttons: vec![],
            media_file_id: None,
            media_type: None,
            admin_only: false,
            user_only: false,
            protect: false,
            replytag: false,
        }
    }

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

/// Filters configuration for a group.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FiltersConfig {
    /// List of filters
    #[serde(default)]
    pub filters: Vec<Filter>,
}

impl FiltersConfig {
    /// Add a new filter. Replaces if trigger already exists.
    pub fn add_filter(&mut self, filter: Filter) {
        // Remove existing filter with same trigger
        self.filters.retain(|f| f.trigger.to_lowercase() != filter.trigger.to_lowercase());
        self.filters.push(filter);
    }

    /// Remove a filter by trigger.
    pub fn remove_filter(&mut self, trigger: &str) -> bool {
        let trigger_lower = trigger.to_lowercase();
        let initial_len = self.filters.len();
        self.filters.retain(|f| f.trigger.to_lowercase() != trigger_lower);
        self.filters.len() < initial_len
    }

    /// Get a filter by trigger.
    pub fn get_filter(&self, trigger: &str) -> Option<&Filter> {
        let trigger_lower = trigger.to_lowercase();
        self.filters.iter().find(|f| f.trigger.to_lowercase() == trigger_lower)
    }

    /// Find filters that match a message.
    pub fn find_matching(&self, message: &str) -> Vec<&Filter> {
        self.filters.iter().filter(|f| f.matches(message)).collect()
    }

    /// Clear all filters.
    pub fn clear_all(&mut self) -> usize {
        let count = self.filters.len();
        self.filters.clear();
        count
    }
}

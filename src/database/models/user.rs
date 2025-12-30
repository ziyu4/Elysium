//! User data model for caching user information.
//!
//! Stores user data from Telegram for username resolution.

use serde::{Deserialize, Serialize};
use teloxide::types::User;

/// Cached user data from Telegram.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedUser {
    /// Telegram user ID.
    pub user_id: u64,
    /// Username without @ (lowercase for matching).
    pub username: Option<String>,
    /// Original username (preserving case for display).
    pub username_display: Option<String>,
    /// First name.
    pub first_name: String,
    /// Last name.
    pub last_name: Option<String>,
    /// Unix timestamp of last update.
    pub updated_at: i64,
}

impl CachedUser {
    /// Create a new CachedUser from Telegram User.
    pub fn from_telegram(user: &User) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            user_id: user.id.0,
            username: user.username.as_ref().map(|u| u.to_lowercase()),
            username_display: user.username.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            updated_at: now,
        }
    }

    /// Check if user data has changed compared to another user.
    pub fn has_changed(&self, other: &User) -> bool {
        let new_username = other.username.as_ref().map(|u| u.to_lowercase());
        self.username != new_username
            || self.first_name != other.first_name
            || self.last_name != other.last_name
    }

    /// Get display name (first name or username).
    pub fn display_name(&self) -> &str {
        &self.first_name
    }
}

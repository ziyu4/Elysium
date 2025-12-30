//! AFK (Away From Keyboard) model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AFK status for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfkStatus {
    /// Reason for being AFK (optional)
    pub reason: Option<String>,
    /// When the user went AFK
    pub since: DateTime<Utc>,
    /// User's first name for display
    #[serde(default)]
    pub first_name: String,
    /// User's username (optional, without @)
    #[serde(default)]
    pub username: Option<String>,
}

impl AfkStatus {
    /// Create a new AFK status.
    pub fn new(reason: Option<String>, first_name: String, username: Option<String>) -> Self {
        Self {
            reason,
            since: Utc::now(),
            first_name,
            username,
        }
    }

    /// Get duration since AFK started in seconds.
    pub fn duration_secs(&self) -> u64 {
        let now = Utc::now();
        (now - self.since).num_seconds().max(0) as u64
    }
}

/// AFK configuration for a group.
/// Uses String keys for MongoDB compatibility (BSON doesn't support integer map keys).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AfkConfig {
    /// Map of user_id (as string) to AFK status
    #[serde(default)]
    pub users: HashMap<String, AfkStatus>,
    /// Map of username (lowercase, without @) to user_id for quick lookup
    #[serde(default)]
    pub username_to_id: HashMap<String, u64>,
}

impl AfkConfig {
    /// Set a user as AFK.
    pub fn set_afk(&mut self, user_id: u64, reason: Option<String>, first_name: String, username: Option<String>) {
        // Store username mapping if available
        if let Some(ref uname) = username {
            self.username_to_id.insert(uname.to_lowercase(), user_id);
        }
        self.users.insert(user_id.to_string(), AfkStatus::new(reason, first_name, username));
    }

    /// Remove AFK status for a user.
    pub fn remove_afk(&mut self, user_id: u64) -> Option<AfkStatus> {
        let status = self.users.remove(&user_id.to_string());
        // Also remove username mapping
        if let Some(ref s) = status
            && let Some(ref uname) = s.username {
                self.username_to_id.remove(&uname.to_lowercase());
            }
        status
    }

    /// Check if a user is AFK.
    pub fn is_afk(&self, user_id: u64) -> bool {
        self.users.contains_key(&user_id.to_string())
    }

    /// Get AFK status for a user.
    pub fn get_afk(&self, user_id: u64) -> Option<&AfkStatus> {
        self.users.get(&user_id.to_string())
    }

    /// Get AFK status by username (without @, case-insensitive).
    pub fn get_afk_by_username(&self, username: &str) -> Option<(u64, &AfkStatus)> {
        let normalized = username.to_lowercase();
        if let Some(&user_id) = self.username_to_id.get(&normalized)
            && let Some(status) = self.users.get(&user_id.to_string()) {
                return Some((user_id, status));
            }
        None
    }
}

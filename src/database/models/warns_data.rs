//! Warns configuration model for separate collection.

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use super::warn::{WarnConfig, UserWarns};

/// Warns data stored in its own collection.
/// Contains both configuration and per-user warnings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarnsData {
    /// MongoDB document ID
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Telegram chat ID (indexed)
    pub chat_id: i64,

    /// Warning configuration
    #[serde(default)]
    pub config: WarnConfig,

    /// Per-user warnings
    #[serde(default)]
    pub user_warns: Vec<UserWarns>,
}

impl Default for WarnsData {
    fn default() -> Self {
        Self {
            id: None,
            chat_id: 0,
            config: WarnConfig::default(),
            user_warns: Vec::new(),
        }
    }
}

impl WarnsData {
    /// Create new warns data for a chat.
    pub fn new(chat_id: i64) -> Self {
        Self {
            chat_id,
            ..Default::default()
        }
    }

    /// Get or create user warns entry.
    pub fn get_or_create_user(&mut self, user_id: u64) -> &mut UserWarns {
        if let Some(idx) = self.user_warns.iter().position(|u| u.user_id == user_id) {
            &mut self.user_warns[idx]
        } else {
            self.user_warns.push(UserWarns::new(user_id));
            self.user_warns.last_mut().unwrap()
        }
    }

    /// Get user warns if exists.
    pub fn get_user(&self, user_id: u64) -> Option<&UserWarns> {
        self.user_warns.iter().find(|u| u.user_id == user_id)
    }

    /// Remove a user's warns entirely.
    pub fn remove_user(&mut self, user_id: u64) -> bool {
        if let Some(idx) = self.user_warns.iter().position(|u| u.user_id == user_id) {
            self.user_warns.remove(idx);
            true
        } else {
            false
        }
    }
}

//! MessageContext model for per-message lookups.
//!
//! Lightweight struct containing only data needed for every-message checks

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use super::antiflood::AntifloodConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContext {
    /// MongoDB document ID
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Telegram chat ID (indexed)
    pub chat_id: i64,

    /// Group title (cached for reference)
    #[serde(default)]
    pub title: Option<String>,

    /// Approved user IDs (bypass antiflood)
    #[serde(default)]
    pub approved_users: Vec<u64>,

    /// Antiflood configuration
    #[serde(default)]
    pub antiflood: AntifloodConfig,
}

impl MessageContext {
    /// Create new context with defaults.
    pub fn new(chat_id: i64) -> Self {
        Self {
            id: None,
            chat_id,
            title: None,
            approved_users: Vec::new(),
            antiflood: AntifloodConfig::default(),
        }
    }

    /// Check if a user is approved.
    pub fn is_approved(&self, user_id: u64) -> bool {
        self.approved_users.contains(&user_id)
    }

    /// Approve a user.
    pub fn approve_user(&mut self, user_id: u64) -> bool {
        if !self.approved_users.contains(&user_id) {
            self.approved_users.push(user_id);
            true
        } else {
            false
        }
    }

    /// Unapprove a user.
    pub fn unapprove_user(&mut self, user_id: u64) -> bool {
        if let Some(pos) = self.approved_users.iter().position(|&id| id == user_id) {
            self.approved_users.remove(pos);
            true
        } else {
            false
        }
    }

    /// Clear all approved users.
    pub fn unapprove_all(&mut self) -> usize {
        let count = self.approved_users.len();
        self.approved_users.clear();
        count
    }
}

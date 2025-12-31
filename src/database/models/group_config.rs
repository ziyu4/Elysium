//! Lightweight group configuration.
//!
//! Replaces the monolithic GroupSettings.
//! Only contains top-level configuration, not heavy content.

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use super::afk::AfkConfig;
use super::antiflood::AntifloodConfig;
use super::bye::ByeConfig;
use super::rules::RulesConfig;
use super::welcome::WelcomeConfig;
use super::warn::{WarnConfig, UserWarns};

/// Lightweight group configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    /// MongoDB document ID
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Telegram chat ID
    pub chat_id: i64,

    /// Group title (cached for reference)
    #[serde(default)]
    pub title: Option<String>,

    /// Antiflood configuration
    #[serde(default)]
    pub antiflood: AntifloodConfig,

    /// Welcome configuration
    #[serde(default)]
    pub welcome: WelcomeConfig,

    /// Rules configuration (Just text, no heavy arrays)
    #[serde(default)]
    pub rules: RulesConfig,

    /// AFK configuration (Global settings only, not status)
    #[serde(default)]
    pub afk: AfkConfig,

    /// Goodbye configuration
    #[serde(default)]
    pub bye: ByeConfig,

    /// Warning configuration (Settings only)
    #[serde(default)]
    pub warn: WarnConfig,

    /// Approved user IDs (bypass antiflood)
    #[serde(default)]
    pub approved_users: Vec<u64>,

    /// Per-user warning data (legacy compatibility)
    #[serde(default)]
    pub user_warns: Vec<UserWarns>,
}

impl GroupConfig {
    /// Create new group config with defaults.
    pub fn new(chat_id: i64) -> Self {
        Self {
            id: None,
            chat_id,
            title: None,
            antiflood: AntifloodConfig::default(),
            welcome: WelcomeConfig::default(),
            rules: RulesConfig::default(),
            afk: AfkConfig::default(),
            bye: ByeConfig::default(),
            warn: WarnConfig::default(),
            approved_users: Vec::new(),
            user_warns: Vec::new(),
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


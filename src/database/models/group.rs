//! Group settings model.

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use super::afk::AfkConfig;
use super::antiflood::AntifloodConfig;
use super::bye::ByeConfig;
use super::filter::FiltersConfig;
use super::notes::NotesConfig;
use super::rules::RulesConfig;
use super::warn::{WarnConfig, UserWarns};
use super::welcome::WelcomeConfig;

/// Complete group settings document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSettings {
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

    /// Rules configuration
    #[serde(default)]
    pub rules: RulesConfig,

    /// Notes configuration
    #[serde(default)]
    pub notes: NotesConfig,

    /// Filters configuration (auto-reply triggers)
    #[serde(default)]
    pub filters: FiltersConfig,

    /// AFK configuration
    #[serde(default)]
    pub afk: AfkConfig,

    /// Goodbye configuration
    #[serde(default)]
    pub bye: ByeConfig,

    /// Warning configuration
    #[serde(default)]
    pub warn: WarnConfig,

    /// User warnings (per-user in this group)
    #[serde(default)]
    pub user_warns: Vec<UserWarns>,

    /// Approved user IDs (bypass antiflood)
    #[serde(default)]
    pub approved_users: Vec<u64>,
}

impl GroupSettings {
    /// Create new group settings with defaults.
    pub fn new(chat_id: i64) -> Self {
        Self {
            id: None,
            chat_id,
            title: None,
            antiflood: AntifloodConfig::default(),
            welcome: WelcomeConfig::default(),
            rules: RulesConfig::default(),
            notes: NotesConfig::default(),
            filters: FiltersConfig::default(),
            afk: AfkConfig::default(),
            bye: ByeConfig::default(),
            warn: WarnConfig::default(),
            user_warns: Vec::new(),
            approved_users: Vec::new(),
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

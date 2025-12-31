//! Warning system models.
//!
//! Data structures for user warnings in groups.

use serde::{Deserialize, Serialize};

/// Warn mode - action when limit reached.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WarnMode {
    #[default]
    Ban,
    Mute,
    Kick,
    TBan,
    TMute,
}

impl WarnMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ban" => Some(Self::Ban),
            "mute" => Some(Self::Mute),
            "kick" => Some(Self::Kick),
            "tban" => Some(Self::TBan),
            "tmute" => Some(Self::TMute),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ban => "ban",
            Self::Mute => "mute",
            Self::Kick => "kick",
            Self::TBan => "tban",
            Self::TMute => "tmute",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Ban => "Ban permanen",
            Self::Mute => "Mute permanen",
            Self::Kick => "Kick (bisa join lagi)",
            Self::TBan => "Ban sementara",
            Self::TMute => "Mute sementara",
        }
    }
}

/// Warning configuration per group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarnConfig {
    /// Maximum warnings before action (default: 3)
    #[serde(default = "default_limit")]
    pub limit: u32,

    /// Action when limit reached
    #[serde(default)]
    pub mode: WarnMode,

    /// Warn expiry in seconds (None = permanent)
    #[serde(default)]
    pub warn_time_secs: Option<u64>,

    /// Duration for tban/tmute in seconds (default: 1 day)
    #[serde(default = "default_action_duration")]
    pub action_duration_secs: u64,
}

fn default_limit() -> u32 {
    3
}

fn default_action_duration() -> u64 {
    86400 // 1 day
}

impl Default for WarnConfig {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            mode: WarnMode::default(),
            warn_time_secs: None,
            action_duration_secs: default_action_duration(),
        }
    }
}

/// Individual warning entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    /// Reason for the warning (optional)
    pub reason: Option<String>,
    /// Admin who issued the warning
    pub warned_by: u64,
    /// Unix timestamp when warning was issued
    pub timestamp: i64,
}

impl Warning {
    pub fn new(reason: Option<String>, warned_by: u64) -> Self {
        Self {
            reason,
            warned_by,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if warning has expired.
    pub fn is_expired(&self, warn_time_secs: Option<u64>) -> bool {
        match warn_time_secs {
            Some(ttl) => {
                let now = chrono::Utc::now().timestamp();
                (now - self.timestamp) >= ttl as i64
            }
            None => false, // No expiry
        }
    }
}

/// Per-user warnings in a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWarns {
    pub user_id: u64,
    pub warnings: Vec<Warning>,
}

impl UserWarns {
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            warnings: Vec::new(),
        }
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }

    /// Remove the latest warning.
    pub fn remove_latest(&mut self) -> Option<Warning> {
        self.warnings.pop()
    }

    /// Get active (non-expired) warnings count.
    pub fn active_count(&self, warn_time_secs: Option<u64>) -> usize {
        self.warnings
            .iter()
            .filter(|w| !w.is_expired(warn_time_secs))
            .count()
    }
}

//! Antiflood configuration models.

use serde::{Deserialize, Serialize};

/// Penalty type for antiflood violations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FloodPenalty {
    /// Warn the user (no action, just message)
    Warn,
    /// Mute the user for a duration
    Mute,
    /// Kick the user (can rejoin)
    Kick,
    /// Ban temporarily
    TempBan,
    /// Ban permanently
    Ban,
}

impl Default for FloodPenalty {
    fn default() -> Self {
        Self::Mute
    }
}

/// Antiflood configuration for a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntifloodConfig {
    /// Whether antiflood is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Maximum messages allowed in the time window
    #[serde(default = "default_max_messages")]
    pub max_messages: u32,

    /// Time window in seconds
    #[serde(default = "default_time_window")]
    pub time_window_secs: u32,

    /// Penalty for flooding
    #[serde(default)]
    pub penalty: FloodPenalty,

    /// Duration for mute/tempban in seconds (0 = forever for ban)
    #[serde(default = "default_penalty_duration")]
    pub penalty_duration_secs: u64,

    /// Number of warnings before penalty (0 = immediate penalty)
    #[serde(default)]
    pub warnings_before_penalty: u32,
}

fn default_max_messages() -> u32 {
    5
}

fn default_time_window() -> u32 {
    5
}

fn default_penalty_duration() -> u64 {
    300 // 5 minutes
}

impl Default for AntifloodConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_messages: 5,
            time_window_secs: 5,
            penalty: FloodPenalty::Mute,
            penalty_duration_secs: 300,
            warnings_before_penalty: 1,
        }
    }
}

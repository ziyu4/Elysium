//! Database module exports.

pub mod afk;
pub mod antiflood;
pub mod bye;
pub mod common;
pub mod filter; // Deprecated, use db_filter
pub mod group;  // Deprecated, use group_config
pub mod notes;  // Deprecated, use db_note
pub mod rules;
pub mod user;
pub mod warn;
pub mod welcome;

pub mod db_filter;
pub mod db_note;
pub mod group_config; // Deprecated, use message_context

// New decentralized models
pub mod message_context;
pub mod welcome_settings;
pub mod bye_settings;
pub mod rules_settings;
pub mod warns_data;

pub use antiflood::{AntifloodConfig, FloodPenalty};
pub use bye::ByeConfig;
pub use common::InlineButton;
 // Legacy
 // Legacy
 // Legacy
pub use user::CachedUser;
pub use warn::{WarnConfig, WarnMode, UserWarns, Warning};
pub use welcome::WelcomeConfig;

pub use db_filter::DbFilter;
pub use db_note::DbNote;
 // Legacy

// New exports
pub use message_context::MessageContext;
pub use welcome_settings::WelcomeSettings;
pub use bye_settings::ByeSettings;
pub use rules_settings::RulesSettings;
pub use warns_data::WarnsData;


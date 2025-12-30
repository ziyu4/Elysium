//! Database models module.
//!
//! All models are split into separate files for maintainability.

mod afk;
mod antiflood;
mod bye;
mod common;
mod filter;
mod group;
mod notes;
mod rules;
mod user;
mod warn;
mod welcome;

// Re-export all public types
pub use afk::AfkStatus;
pub use antiflood::FloodPenalty;
pub use bye::ByeConfig;
pub use common::InlineButton;
pub use filter::{Filter, MatchType};
pub use group::GroupSettings;
pub use notes::{Note, NoteTags};
pub use user::CachedUser;
pub use warn::{WarnConfig, WarnMode, Warning, UserWarns};
pub use welcome::WelcomeConfig;

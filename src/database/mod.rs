//! Database module exports.

pub mod models;
mod mongo;
pub mod repository;
mod users;

pub use models::*;
pub use mongo::Database;
pub use repository::{
    FilterRepository,
    NoteRepository,
    MessageContextRepository,
    WelcomeRepository,
    ByeRepository,
    RulesRepository,
    WarnsRepository,
};
pub use users::UserRepo;

// Re-export for backwards compatibility
pub use models::db_filter::{DbFilter, MatchType};

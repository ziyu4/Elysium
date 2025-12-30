//! Permission system for checking user roles.
//!
//! This module provides utilities for checking if a user is an admin,
//! owner, or has specific permissions in a chat.
//!
//! ## Features
//!
//! - Cached admin lookups (reduces API hits)
//! - Support for checking specific permissions
//! - Owner detection
//!
//! ## Usage
//!
//! ```rust
//! let perms = Permissions::new(bot.clone(), state.cache.clone());
//!
//! // Check if user is admin
//! if perms.is_admin(chat_id, user_id).await? {
//!     // ...
//! }
//!
//! // Check for specific permission
//! if perms.can_delete_messages(chat_id, user_id).await? {
//!     // ...
//! }
//! ```

mod checker;

pub use checker::Permissions;

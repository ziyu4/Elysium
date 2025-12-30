//! Cache module - Modular caching system using Moka.
//!
//! This module provides a registry-based caching system that allows
//! plugins and events to easily manage their own caches.
//!
//! ## Architecture
//!
//! The cache system follows a registry pattern:
//! - `CacheRegistry` - Central registry holding all named caches
//! - `CacheBuilder` - Builder for creating typed caches with custom config
//! - Individual caches are created per domain (users, groups, settings, etc.)
//!
//! ## Usage
//!
//! ```rust
//! // Create a cache for users
//! let users_cache = registry.get_or_create::<i64, User>("users", CacheConfig::default());
//!
//! // Use the cache
//! users_cache.insert(user_id, user);
//! let user = users_cache.get(&user_id);
//! ```

mod config;
mod registry;
mod typed;

pub use config::CacheConfig;
pub use registry::CacheRegistry;
pub use typed::TypedCache;

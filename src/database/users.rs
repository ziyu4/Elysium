//! User repository with cache-first architecture.
//!
//! Provides user storage and resolution with dual-index caching:
//! - By user ID (primary)
//! - By username (for @username resolution)

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use mongodb::bson::doc;
use mongodb::Collection;
use teloxide::types::User;
use tokio::spawn;
use tracing::{debug, warn};

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use super::models::CachedUser;
use super::Database;

/// Repository for user data with dual-index caching.
pub struct UserRepo {
    collection: Collection<CachedUser>,
    cache_by_id: TypedCache<u64, CachedUser>,
    cache_by_username: TypedCache<String, u64>, // username (lowercase) -> user_id
}

impl UserRepo {
    /// Create a new UserRepo with caching.
    pub fn new(db: &Database, cache: &CacheRegistry) -> Self {
        let cache_by_id = cache.get_or_create(
            "users_by_id",
            CacheConfig {
                max_capacity: 10_000,
                ttl: Some(Duration::from_secs(3600)), // 1 hour
                ..Default::default()
            },
        );

        let cache_by_username = cache.get_or_create(
            "users_by_username",
            CacheConfig {
                max_capacity: 10_000,
                ttl: Some(Duration::from_secs(1800)), // 30 min (shorter for username changes)
                ..Default::default()
            },
        );

        Self {
            collection: db.collection("users"),
            cache_by_id,
            cache_by_username,
        }
    }

    /// Upsert user data (update or insert).
    /// Updates cache immediately.
    pub async fn upsert(&self, user: &User) -> Result<()> {
        let user_id = user.id.0;

        // Check if data changed (skip unnecessary writes)
        if let Some(cached) = self.cache_by_id.get(&user_id) {
            if !cached.has_changed(user) {
                return Ok(());
            }
            // Remove old username from cache if changed
            if let Some(old_username) = &cached.username {
                let new_username = user.username.as_ref().map(|u| u.to_lowercase());
                if Some(old_username.clone()) != new_username {
                    self.cache_by_username.invalidate(old_username);
                }
            }
        }

        let cached_user = CachedUser::from_telegram(user);

        // Update caches
        self.cache_by_id.insert(user_id, cached_user.clone());
        if let Some(username) = &cached_user.username {
            self.cache_by_username.insert(username.clone(), user_id);
        }

        // Persist to DB
        let filter = doc! { "user_id": user_id as i64 };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter, &cached_user)
            .with_options(options)
            .await?;

        debug!("Upserted user {} (@{:?})", user_id, cached_user.username);
        Ok(())
    }

    /// Upsert user in background (non-blocking).
    pub fn upsert_background(self: Arc<Self>, user: User) {
        spawn(async move {
            if let Err(e) = self.upsert(&user).await {
                warn!("Failed to upsert user {}: {}", user.id, e);
            }
        });
    }

    /// Get user by ID.
    pub async fn get_by_id(&self, user_id: u64) -> Result<Option<CachedUser>> {
        // Check cache
        if let Some(user) = self.cache_by_id.get(&user_id) {
            return Ok(Some(user));
        }

        // Fetch from DB
        let filter = doc! { "user_id": user_id as i64 };
        let result = self.collection.find_one(filter).await?;

        // Update cache
        if let Some(user) = &result {
            self.cache_by_id.insert(user_id, user.clone());
            if let Some(username) = &user.username {
                self.cache_by_username.insert(username.clone(), user_id);
            }
        }

        Ok(result)
    }

    /// Get user by username (case-insensitive).
    pub async fn get_by_username(&self, username: &str) -> Result<Option<CachedUser>> {
        let username_lower = username.to_lowercase();

        // Check username -> ID cache
        if let Some(user_id) = self.cache_by_username.get(&username_lower) {
            // Then fetch full user from ID cache
            if let Some(user) = self.cache_by_id.get(&user_id) {
                return Ok(Some(user));
            }
            // ID in cache but user not? Fetch from DB
            return self.get_by_id(user_id).await;
        }

        // Fetch from DB by username
        let filter = doc! { "username": &username_lower };
        let result = self.collection.find_one(filter).await?;

        // Update caches
        if let Some(user) = &result {
            self.cache_by_id.insert(user.user_id, user.clone());
            self.cache_by_username.insert(username_lower, user.user_id);
        }

        Ok(result)
    }

    /// Resolve username to UserId (convenience method).
    pub async fn resolve_username(&self, username: &str) -> Option<u64> {
        let username_clean = username.trim_start_matches('@');
        match self.get_by_username(username_clean).await {
            Ok(Some(user)) => Some(user.user_id),
            _ => None,
        }
    }
}

impl Clone for UserRepo {
    fn clone(&self) -> Self {
        Self {
            collection: self.collection.clone(),
            cache_by_id: self.cache_by_id.clone(),
            cache_by_username: self.cache_by_username.clone(),
        }
    }
}

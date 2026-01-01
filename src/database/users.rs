//! User repository with cache-first architecture.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use mongodb::bson::doc;
use mongodb::Collection;
use teloxide::types::User;
use tokio::spawn;
use tracing::warn;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use crate::database::models::CachedUser;
use crate::database::Database;

/// Repository for user data with dual-index caching.
pub struct UserRepo {
    collection: Collection<CachedUser>,
    cache_by_id: TypedCache<u64, CachedUser>,
    cache_by_username: TypedCache<String, u64>, // username (lowercase) -> user_id
    debounce_cache: TypedCache<u64, ()>,         // Skip updates if recently processed
}

impl UserRepo {
    /// Create a new UserRepo with caching.
    pub fn new(db: &Database, cache: &CacheRegistry) -> Self {
        let cache_by_id = cache.get_or_create(
            "users_by_id",
            CacheConfig::default()
                .ttl(Duration::from_secs(3600))
                .max_capacity(10_000),
        );

        let cache_by_username = cache.get_or_create(
            "users_by_username",
            CacheConfig::default()
                .ttl(Duration::from_secs(1800))
                .max_capacity(10_000),
        );

        // Debounce cache - skip updates for users processed in last 30 seconds
        let debounce_cache = cache.get_or_create(
            "users_debounce",
            CacheConfig::default()
                .ttl(Duration::from_secs(30))
                .max_capacity(50_000),
        );

        Self {
            collection: db.collection("users"),
            cache_by_id,
            cache_by_username,
            debounce_cache,
        }
    }

    /// Upsert user data (update or insert).
    /// Uses debounce to skip redundant updates within 30 seconds.
    /// Preserves internal state (AFK, lang) when updating.
    pub async fn upsert(&self, user: &User) -> Result<()> {
        let user_id = user.id.0;

        // DEBOUNCE: Skip if this user was processed recently
        if self.debounce_cache.contains(&user_id) {
            return Ok(());
        }

        // Check cache first
        if let Some(mut existing) = self.cache_by_id.get(&user_id) {
            if !existing.has_changed(user) {
                // Data unchanged - just mark as debounced and return
                self.debounce_cache.insert(user_id, ());
                return Ok(());
            }

            // Data changed - save old username for cache invalidation
            let old_username = existing.username.clone();
            
            // Update with new Telegram data (preserves AFK, lang)
            existing.update_from_telegram(user);

            // Invalidate old username cache if changed
            if let Some(old) = &old_username {
                if existing.username.as_ref() != Some(old) {
                    self.cache_by_username.invalidate(old);
                }
            }

            // Update caches
            self.cache_by_id.insert(user_id, existing.clone());
            if let Some(username) = &existing.username {
                self.cache_by_username.insert(username.clone(), user_id);
            }
            self.debounce_cache.insert(user_id, ());

            return self.persist_to_db(&existing).await;
        }

        // Not in cache - check DB to preserve internal state
        let cached_user = if let Ok(Some(mut db_user)) = self.get_by_id_internal(user_id).await {
            db_user.update_from_telegram(user);
            db_user
        } else {
            CachedUser::from_telegram(user)
        };

        // Update caches
        self.cache_by_id.insert(user_id, cached_user.clone());
        if let Some(username) = &cached_user.username {
            self.cache_by_username.insert(username.clone(), user_id);
        }
        self.debounce_cache.insert(user_id, ());

        self.persist_to_db(&cached_user).await
    }

    /// Persist user to database.
    async fn persist_to_db(&self, user: &CachedUser) -> Result<()> {
        let filter = doc! { "user_id": user.user_id as i64 };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter, user)
            .with_options(options)
            .await?;

        Ok(())
    }

    /// Upsert user in background.
    pub fn upsert_background(self: Arc<Self>, user: User) {
        spawn(async move {
            if let Err(e) = self.upsert(&user).await {
                warn!("Failed to upsert user {}: {}", user.id, e);
            }
        });
    }

    /// Set AFK status.
    pub async fn set_afk(&self, user_id: u64, reason: Option<String>) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let reason_val = reason.unwrap_or_else(|| "ㅤ".to_string());

        // Update DB
        let filter = doc! { "user_id": user_id as i64 };
        let update = doc! { 
            "$set": { 
                "afk_reason": &reason_val,
                "afk_time": now
            }
        };
        self.collection.update_one(filter, update).await?;

        // Update Cache
        if let Some(mut user) = self.cache_by_id.get(&user_id) {
            user.afk_reason = Some(reason_val);
            user.afk_time = Some(now);
            self.cache_by_id.insert(user_id, user);
        } else {
            // Force reload
            let _ = self.get_by_id(user_id).await;
        }

        Ok(())
    }

    /// Remove AFK status.
    pub async fn remove_afk(&self, user_id: u64) -> Result<()> {
        // Update DB
        let filter = doc! { "user_id": user_id as i64 };
        let update = doc! { 
            "$unset": { 
                "afk_reason": "",
                "afk_time": ""
            }
        };
        self.collection.update_one(filter, update).await?;

        // Update Cache
        if let Some(mut user) = self.cache_by_id.get(&user_id) {
            user.afk_reason = None;
            user.afk_time = None;
            self.cache_by_id.insert(user_id, user);
        }

        Ok(())
    }

    /// Get user by ID.
    pub async fn get_by_id(&self, user_id: u64) -> Result<Option<CachedUser>> {
        if let Some(user) = self.cache_by_id.get(&user_id) {
            return Ok(Some(user));
        }
        self.get_by_id_internal(user_id).await
    }
    
    async fn get_by_id_internal(&self, user_id: u64) -> Result<Option<CachedUser>> {
        let filter = doc! { "user_id": user_id as i64 };
        let result = self.collection.find_one(filter).await?;

        if let Some(user) = &result {
            self.cache_by_id.insert(user_id, user.clone());
            if let Some(username) = &user.username {
                self.cache_by_username.insert(username.clone(), user_id);
            }
        }
        Ok(result)
    }

    /// Get user by username.
    pub async fn get_by_username(&self, username: &str) -> Result<Option<CachedUser>> {
        let username_lower = username.to_lowercase();
        if let Some(user_id) = self.cache_by_username.get(&username_lower) {
            if let Some(user) = self.cache_by_id.get(&user_id) {
                return Ok(Some(user));
            }
            return self.get_by_id(user_id).await;
        }

        let filter = doc! { "username": &username_lower };
        let result = self.collection.find_one(filter).await?;

        if let Some(user) = &result {
            self.cache_by_id.insert(user.user_id, user.clone());
            self.cache_by_username.insert(username_lower, user.user_id);
        }

        Ok(result)
    }

    /// Set user language.
    pub async fn set_lang(&self, user_id: u64, lang: String) -> Result<()> {
        let mut user = match self.get_by_id(user_id).await? {
            Some(u) => u,
            None => return Ok(()), // Should probably error or upsert, but for now ignore if unknown
        };

        user.lang = Some(lang.clone());
        self.cache_by_id.insert(user_id, user);

        let filter = doc! { "user_id": user_id as i64 };
        let update = doc! { "$set": { "lang": lang } };
        
        self.collection.update_one(filter, update).await?;
        Ok(())
    }
}

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

        Self {
            collection: db.collection("users"),
            cache_by_id,
            cache_by_username,
        }
    }

    /// Upsert user data (update or insert).
    /// Preserves existing AFK status if not changing.
    pub async fn upsert(&self, user: &User) -> Result<()> {
        let user_id = user.id.0;
        let mut cached_user = CachedUser::from_telegram(user);

        // Check if data changed
        if let Some(existing) = self.cache_by_id.get(&user_id) {
            // Preserve AFK status from existing cache/db
            cached_user.afk_reason = existing.afk_reason.clone();
            cached_user.afk_time = existing.afk_time;
            // Preserve Lang if not explicitly set (which it isn't from telegram update)
            cached_user.lang = existing.lang.clone();

            if !existing.has_changed(user) {
                // If core data hasn't changed, we don't need to write to DB
                // BUT we might need to update Last Seen? Let's skip for efficiency unless TTL is low.
                return Ok(());
            }
            
            // Invalidate old username if changed
            if let Some(old_username) = &existing.username {
                let new_username = user.username.as_ref().map(|u| u.to_lowercase());
                if Some(old_username.clone()) != new_username {
                    self.cache_by_username.invalidate(old_username);
                }
            }
        } else {
             // If not in cache, try to get from DB to preserve AFK?
             // Or just assume new/overwrite if not active.
             // For robustness, usually we merge. But standard upsert is fine for now.
             // If they were AFK and we rebooted, loading from DB resolves this.
             if let Ok(Some(db_user)) = self.get_by_id_internal(user_id).await {
                 cached_user.afk_reason = db_user.afk_reason.clone();
                 cached_user.afk_time = db_user.afk_time;
                 cached_user.lang = db_user.lang.clone();
             }
        }

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
        let reason_val = reason.unwrap_or_else(|| "AFK".to_string());

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

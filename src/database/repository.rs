//! Group settings repository.
//!
//! Handles CRUD operations for group settings in MongoDB.
//! Uses caching to minimize database access.

use std::time::Duration;

use anyhow::Result;
use mongodb::bson::doc;
use mongodb::Collection;
use tracing::debug;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use super::models::GroupSettings;
use super::Database;

/// Repository for group settings.
pub struct GroupSettingsRepo {
    collection: Collection<GroupSettings>,
    cache: Option<TypedCache<i64, GroupSettings>>,
}

impl GroupSettingsRepo {
    /// Create a new repository instance with caching.
    pub fn new(db: &Database, cache: &CacheRegistry) -> Self {
        let settings_cache = cache.get_or_create(
            "group_settings",
            CacheConfig {
                max_capacity: 1000,
                ttl: Some(Duration::from_secs(300)), // 5 mins
                ..Default::default()
            },
        );

        Self {
            collection: db.collection("group_settings"),
            cache: Some(settings_cache),
        }
    }

    /// Create a new repository instance without caching (for testing/maintenance).
    #[allow(dead_code)]
    pub fn new_no_cache(db: &Database) -> Self {
        Self {
            collection: db.collection("group_settings"),
            cache: None,
        }
    }

    /// Get group settings, creating default if not exists.
    pub async fn get_or_create(&self, chat_id: i64) -> Result<GroupSettings> {
        if let Some(settings) = self.get(chat_id).await? {
            return Ok(settings);
        }

        // Create default settings
        let settings = GroupSettings::new(chat_id);
        self.save_to_db(&settings).await?;
        
        // Update cache
        if let Some(cache) = &self.cache {
            cache.insert(chat_id, settings.clone());
        }

        Ok(settings)
    }

    /// Get group settings by chat ID (returning Option).
    pub async fn get(&self, chat_id: i64) -> Result<Option<GroupSettings>> {
        // Check cache first
        if let Some(cache) = &self.cache
            && let Some(settings) = cache.get(&chat_id) {
                return Ok(Some(settings));
            }

        // Fetch from DB
        let settings = self.get_from_db(chat_id).await?;

        // Update cache
        if let Some(s) = &settings
            && let Some(cache) = &self.cache {
                cache.insert(chat_id, s.clone());
            }

        Ok(settings)
    }

    /// Get directly from DB (internal).
    async fn get_from_db(&self, chat_id: i64) -> Result<Option<GroupSettings>> {
        let filter = doc! { "chat_id": chat_id };
        let result = self.collection.find_one(filter).await?;
        debug!("DB get group settings for {}: {:?}", chat_id, result.is_some());
        Ok(result)
    }

    /// Save group settings (upsert).
    pub async fn save(&self, settings: &GroupSettings) -> Result<()> {
        self.save_to_db(settings).await?;

        // Update cache
        if let Some(cache) = &self.cache {
            cache.insert(settings.chat_id, settings.clone());
        }
        
        Ok(())
    }

    async fn save_to_db(&self, settings: &GroupSettings) -> Result<()> {
        let filter = doc! { "chat_id": settings.chat_id };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter, settings)
            .with_options(options)
            .await?;

        debug!("Saved group settings for {}", settings.chat_id);
        Ok(())
    }

    /// Update a specific field using a document.
    /// Note: This invalidates the cache since we don't have the full object.
    #[allow(dead_code)]
    pub async fn update_field(
        &self,
        chat_id: i64,
        field: &str,
        value: mongodb::bson::Bson,
    ) -> Result<()> {
        let filter = doc! { "chat_id": chat_id };
        let update = doc! { "$set": { field: value } };

        self.collection.update_one(filter, update).await?;
        debug!("Updated {} for chat {}", field, chat_id);

        // Invalidate cache
        if let Some(cache) = &self.cache {
            cache.invalidate(&chat_id);
        }

        Ok(())
    }

    /// Delete group settings.
    pub async fn _delete(&self, chat_id: i64) -> Result<bool> {
        let filter = doc! { "chat_id": chat_id };
        let result = self.collection.delete_one(filter).await?;
        
        // Remove from cache
        if let Some(cache) = &self.cache {
            cache.invalidate(&chat_id);
        }

        debug!("Deleted group settings for {}: {}", chat_id, result.deleted_count > 0);
        Ok(result.deleted_count > 0)
    }

    /// Update group title cache.
    #[allow(dead_code)]
    pub async fn update_title(&self, chat_id: i64, title: &str) -> Result<()> {
        self.update_field(chat_id, "title", mongodb::bson::Bson::String(title.to_string()))
            .await
    }
}

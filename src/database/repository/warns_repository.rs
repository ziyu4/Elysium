//! Warns repository with on-demand caching.
//!
//! Medium TTL (5min) since warns are command-triggered.

use std::time::Duration;

use anyhow::Result;
use mongodb::bson::doc;
use mongodb::Collection;
use tracing::debug;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use crate::database::models::{WarnsData, Warning};
use crate::database::Database;

/// Repository for warns data.
pub struct WarnsRepository {
    collection: Collection<WarnsData>,
    cache: TypedCache<i64, WarnsData>,
}

impl WarnsRepository {
    pub fn new(db: &Database, cache: &CacheRegistry) -> Self {
        let warns_cache = cache.get_or_create(
            "warns_data",
            CacheConfig::with_capacity(3_000)
                .ttl(Duration::from_secs(300)), // 5 minutes
        );

        Self {
            collection: db.collection("warns"),
            cache: warns_cache,
        }
    }

    /// Get warns data, returning None if not exists.
    pub async fn get(&self, chat_id: i64) -> Result<Option<WarnsData>> {
        if let Some(data) = self.cache.get(&chat_id) {
            return Ok(Some(data));
        }

        let filter = doc! { "chat_id": chat_id };
        let result = self.collection.find_one(filter).await?;

        if let Some(d) = &result {
            self.cache.insert(chat_id, d.clone());
        }

        Ok(result)
    }

    /// Get or create warns data with defaults.
    pub async fn get_or_create(&self, chat_id: i64) -> Result<WarnsData> {
        if let Some(data) = self.get(chat_id).await? {
            return Ok(data);
        }

        let data = WarnsData::new(chat_id);
        self.save(&data).await?;
        Ok(data)
    }

    /// Save warns data (upsert).
    pub async fn save(&self, data: &WarnsData) -> Result<()> {
        let filter = doc! { "chat_id": data.chat_id };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter, data)
            .with_options(options)
            .await?;

        self.cache.insert(data.chat_id, data.clone());
        debug!("Saved WarnsData for chat {}", data.chat_id);

        Ok(())
    }

    /// Add a warning to a user.
    pub async fn add_warning(
        &self,
        chat_id: i64,
        user_id: u64,
        reason: Option<String>,
        warned_by: u64,
    ) -> Result<usize> {
        let mut data = self.get_or_create(chat_id).await?;
        let warn_time_secs = data.config.warn_time_secs;
        let user_warns = data.get_or_create_user(user_id);
        user_warns.add_warning(Warning::new(reason, warned_by));
        
        let count = user_warns.active_count(warn_time_secs);
        self.save(&data).await?;
        
        Ok(count)
    }

    /// Remove the latest warning from a user.
    pub async fn remove_warning(&self, chat_id: i64, user_id: u64) -> Result<bool> {
        let mut data = self.get_or_create(chat_id).await?;
        let user_warns = data.get_or_create_user(user_id);
        let removed = user_warns.remove_latest().is_some();
        
        if removed {
            self.save(&data).await?;
        }
        
        Ok(removed)
    }

    /// Reset all warnings for a user.
    pub async fn reset_warnings(&self, chat_id: i64, user_id: u64) -> Result<bool> {
        let mut data = self.get_or_create(chat_id).await?;
        let removed = data.remove_user(user_id);
        
        if removed {
            self.save(&data).await?;
        }
        
        Ok(removed)
    }

    /// Get warning count for a user.
    pub async fn get_warning_count(&self, chat_id: i64, user_id: u64) -> Result<usize> {
        let data = self.get_or_create(chat_id).await?;
        let count = data
            .get_user(user_id)
            .map(|u| u.active_count(data.config.warn_time_secs))
            .unwrap_or(0);
        Ok(count)
    }

    /// Update warn config.
    pub async fn update_config(
        &self,
        chat_id: i64,
        config: crate::database::models::WarnConfig,
    ) -> Result<()> {
        let mut data = self.get_or_create(chat_id).await?;
        data.config = config;
        self.save(&data).await
    }

    /// Invalidate cache.
    pub fn invalidate(&self, chat_id: i64) {
        self.cache.invalidate(&chat_id);
    }
}

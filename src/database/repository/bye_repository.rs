//! Bye/Goodbye repository with lazy loading.
//!
//! Low TTL (5min) since bye events are rare.

use std::time::Duration;

use anyhow::Result;
use mongodb::bson::doc;
use mongodb::Collection;
use tracing::debug;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use crate::database::models::ByeSettings;
use crate::database::Database;

/// Repository for bye settings.
pub struct ByeRepository {
    collection: Collection<ByeSettings>,
    cache: TypedCache<i64, ByeSettings>,
}

impl ByeRepository {
    pub fn new(db: &Database, cache: &CacheRegistry) -> Self {
        let bye_cache = cache.get_or_create(
            "bye_settings",
            CacheConfig::with_capacity(2_000)
                .ttl(Duration::from_secs(300)), // 5 minutes (lazy load)
        );

        Self {
            collection: db.collection("bye"),
            cache: bye_cache,
        }
    }

    /// Get bye settings, returning None if not configured.
    pub async fn get(&self, chat_id: i64) -> Result<Option<ByeSettings>> {
        if let Some(settings) = self.cache.get(&chat_id) {
            return Ok(Some(settings));
        }

        let filter = doc! { "chat_id": chat_id };
        let result = self.collection.find_one(filter).await?;

        if let Some(s) = &result {
            self.cache.insert(chat_id, s.clone());
        }

        Ok(result)
    }

    /// Get or create bye settings with defaults.
    pub async fn get_or_create(&self, chat_id: i64) -> Result<ByeSettings> {
        if let Some(settings) = self.get(chat_id).await? {
            return Ok(settings);
        }

        let settings = ByeSettings::new(chat_id);
        self.save(&settings).await?;
        Ok(settings)
    }

    /// Save bye settings (upsert).
    pub async fn save(&self, settings: &ByeSettings) -> Result<()> {
        let filter = doc! { "chat_id": settings.chat_id };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter, settings)
            .with_options(options)
            .await?;

        self.cache.insert(settings.chat_id, settings.clone());
        debug!("Saved ByeSettings for chat {}", settings.chat_id);

        Ok(())
    }

    /// Update bye message.
    pub async fn set_message(&self, chat_id: i64, message: Option<String>) -> Result<()> {
        let mut settings = self.get_or_create(chat_id).await?;
        settings.message = message;
        self.save(&settings).await
    }

    /// Set enabled state.
    pub async fn set_enabled(&self, chat_id: i64, enabled: bool) -> Result<()> {
        let mut settings = self.get_or_create(chat_id).await?;
        settings.enabled = enabled;
        self.save(&settings).await
    }

    /// Invalidate cache.
    pub fn invalidate(&self, chat_id: i64) {
        self.cache.invalidate(&chat_id);
    }
}

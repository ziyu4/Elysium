//! Welcome repository with lazy loading.
//!
//! Low TTL (5min) since welcome events are rare.

use std::time::Duration;

use anyhow::Result;
use mongodb::bson::doc;
use mongodb::Collection;
use tracing::debug;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use crate::database::models::WelcomeSettings;
use crate::database::Database;

/// Repository for welcome settings.
pub struct WelcomeRepository {
    collection: Collection<WelcomeSettings>,
    cache: TypedCache<i64, WelcomeSettings>,
}

impl WelcomeRepository {
    pub fn new(db: &Database, cache: &CacheRegistry) -> Self {
        let welcome_cache = cache.get_or_create(
            "welcome_settings",
            CacheConfig::with_capacity(2_000)
                .ttl(Duration::from_secs(300)), // 5 minutes (lazy load)
        );

        Self {
            collection: db.collection("welcome"),
            cache: welcome_cache,
        }
    }

    /// Get welcome settings, returning None if not configured.
    pub async fn get(&self, chat_id: i64) -> Result<Option<WelcomeSettings>> {
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

    /// Get or create welcome settings with defaults.
    pub async fn get_or_create(&self, chat_id: i64) -> Result<WelcomeSettings> {
        if let Some(settings) = self.get(chat_id).await? {
            return Ok(settings);
        }

        let settings = WelcomeSettings::new(chat_id);
        self.save(&settings).await?;
        Ok(settings)
    }

    /// Save welcome settings (upsert).
    pub async fn save(&self, settings: &WelcomeSettings) -> Result<()> {
        let filter = doc! { "chat_id": settings.chat_id };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter, settings)
            .with_options(options)
            .await?;

        self.cache.insert(settings.chat_id, settings.clone());
        debug!("Saved WelcomeSettings for chat {}", settings.chat_id);

        Ok(())
    }
}

//! Rules repository with lazy loading.
//!
//! Very low TTL (10min) since rules are rarely accessed.

use std::time::Duration;

use anyhow::Result;
use mongodb::bson::doc;
use mongodb::Collection;
use tracing::debug;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use crate::database::models::RulesSettings;
use crate::database::Database;

/// Repository for rules settings.
pub struct RulesRepository {
    collection: Collection<RulesSettings>,
    cache: TypedCache<i64, RulesSettings>,
}

impl RulesRepository {
    pub fn new(db: &Database, cache: &CacheRegistry) -> Self {
        let rules_cache = cache.get_or_create(
            "rules_settings",
            CacheConfig::with_capacity(2_000)
                .ttl(Duration::from_secs(600)), // 10 minutes (very lazy)
        );

        Self {
            collection: db.collection("rules"),
            cache: rules_cache,
        }
    }

    /// Get rules settings, returning None if not configured.
    pub async fn get(&self, chat_id: i64) -> Result<Option<RulesSettings>> {
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

    /// Get or create rules settings with defaults.
    pub async fn get_or_create(&self, chat_id: i64) -> Result<RulesSettings> {
        if let Some(settings) = self.get(chat_id).await? {
            return Ok(settings);
        }

        let settings = RulesSettings::new(chat_id);
        self.save(&settings).await?;
        Ok(settings)
    }

    /// Save rules settings (upsert).
    pub async fn save(&self, settings: &RulesSettings) -> Result<()> {
        let filter = doc! { "chat_id": settings.chat_id };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter, settings)
            .with_options(options)
            .await?;

        self.cache.insert(settings.chat_id, settings.clone());
        debug!("Saved RulesSettings for chat {}", settings.chat_id);

        Ok(())
    }

    /// Set rules text.
    pub async fn set_rules(&self, chat_id: i64, text: Option<String>) -> Result<()> {
        let mut settings = self.get_or_create(chat_id).await?;
        settings.text = text;
        self.save(&settings).await
    }

    /// Clear rules.
    pub async fn clear_rules(&self, chat_id: i64) -> Result<()> {
        self.set_rules(chat_id, None).await
    }
}

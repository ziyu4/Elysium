//! GroupSettingsRepo - Legacy compatibility wrapper.
//!
//! This provides backwards compatibility during the migration to decentralized repositories.
//! Plugins should migrate to use the specific repositories (MessageContextRepository, 
//! WelcomeRepository, ByeRepository, RulesRepository, WarnsRepository) instead.

use std::time::Duration;

use anyhow::Result;
use mongodb::bson::doc;
use mongodb::Collection;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use crate::database::models::group_config::GroupConfig;
use crate::database::Database;

/// Legacy repository for group settings.
/// 
/// DEPRECATED: Use specific repositories instead:
/// - MessageContextRepository for antiflood + approved_users
/// - WelcomeRepository for welcome settings
/// - ByeRepository for goodbye settings
/// - RulesRepository for rules
/// - WarnsRepository for warnings
#[derive(Clone)]
pub struct GroupSettingsRepo {
    collection: Collection<GroupConfig>,
    cache: TypedCache<i64, GroupConfig>,
}

impl GroupSettingsRepo {
    pub fn new(db: &Database, cache_registry: &CacheRegistry) -> Self {
        let cache = cache_registry.get_or_create(
            "group_config_legacy",
            CacheConfig::with_capacity(5_000)
                .ttl(Duration::from_secs(3600)), // 1 hour
        );

        Self {
            collection: db.collection("group_config"),
            cache,
        }
    }

    /// Get or create group config.
    pub async fn get_or_create(&self, chat_id: i64) -> Result<GroupConfig> {
        if let Some(config) = self.cache.get(&chat_id) {
            return Ok(config);
        }

        let filter = doc! { "chat_id": chat_id };
        let result = self.collection.find_one(filter).await?;

        let config = result.unwrap_or_else(|| GroupConfig::new(chat_id));
        self.cache.insert(chat_id, config.clone());

        Ok(config)
    }

    /// Get group config if exists.
    pub async fn get(&self, chat_id: i64) -> Result<Option<GroupConfig>> {
        if let Some(config) = self.cache.get(&chat_id) {
            return Ok(Some(config));
        }

        let filter = doc! { "chat_id": chat_id };
        let result = self.collection.find_one(filter).await?;

        if let Some(c) = &result {
            self.cache.insert(chat_id, c.clone());
        }

        Ok(result)
    }

    /// Save group config (upsert).
    pub async fn save(&self, config: &GroupConfig) -> Result<()> {
        let filter = doc! { "chat_id": config.chat_id };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter, config)
            .with_options(options)
            .await?;

        self.cache.insert(config.chat_id, config.clone());
        Ok(())
    }

    /// Invalidate cache.
    pub fn invalidate(&self, chat_id: i64) {
        self.cache.invalidate(&chat_id);
    }
}

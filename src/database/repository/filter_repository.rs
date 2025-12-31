//! Filter repository with tiered caching.
//!
//! Implements L1 (Keys), L2 (Content), and L2-Hot (Frequently Accessed) caching.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::Result;
use dashmap::DashMap;
use futures::StreamExt;
use mongodb::bson::{doc, Document};
use mongodb::Collection;
use tracing::debug;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use crate::database::models::DbFilter;
use crate::database::Database;

/// Threshold for promoting to hot cache (access count).
const HOT_PROMOTION_THRESHOLD: u64 = 3;

/// Repository for filters with hot cache tier.
pub struct FilterRepository {
    collection: Collection<DbFilter>,
    /// L1 Cache: ChatID -> Set of Triggers (1 hour TTL)
    triggers_cache: TypedCache<i64, HashSet<String>>,
    /// L2 Cache: (ChatID, Trigger) -> Filter Content (1 min TTL)
    filter_cache: TypedCache<(i64, String), DbFilter>,
    /// L2-Hot Cache: (ChatID, Trigger) -> Filter Content (10 min TTL, promoted items)
    hot_cache: TypedCache<(i64, String), DbFilter>,
    /// Hit counter for promotion decisions
    hit_counter: DashMap<(i64, String), AtomicU64>,
}

impl FilterRepository {
    pub fn new(db: &Database, cache: &CacheRegistry) -> Self {
        let triggers_cache = cache.get_or_create(
            "filter_triggers",
            CacheConfig::default()
                .ttl(Duration::from_secs(3600)) // 1 hour
                .max_capacity(5000),
        );

        let filter_cache = cache.get_or_create(
            "filter_content",
            CacheConfig::hot_data() // 1 min TTL
                .max_capacity(10_000),
        );

        // Hot cache: Longer TTL for frequently accessed items
        let hot_cache = cache.get_or_create(
            "filter_hot",
            CacheConfig::hot_promoted() // 10 min TTL, idle timeout
                .max_capacity(2_000), // Smaller, only for truly hot items
        );

        Self {
            collection: db.collection("filters"),
            triggers_cache,
            filter_cache,
            hot_cache,
            hit_counter: DashMap::with_capacity(1_000),
        }
    }

    /// L1: Get all triggers for a chat.
    pub async fn get_triggers(&self, chat_id: i64) -> Result<HashSet<String>> {
        if let Some(triggers) = self.triggers_cache.get(&chat_id) {
            return Ok(triggers);
        }

        let triggers = self.fetch_triggers_from_db(chat_id).await?;
        self.triggers_cache.insert(chat_id, triggers.clone());
        Ok(triggers)
    }

    /// Helper to fetch triggers from DB.
    async fn fetch_triggers_from_db(&self, chat_id: i64) -> Result<HashSet<String>> {
        let raw_coll: Collection<Document> = self.collection.clone_with_type();
        let filter = doc! { "chat_id": chat_id };
        let options = mongodb::options::FindOptions::builder()
            .projection(doc! { "trigger": 1, "_id": 0 })
            .build();

        let mut cursor = raw_coll.find(filter).with_options(options).await?;
        let mut triggers = HashSet::new();

        while let Some(result) = cursor.next().await {
            if let Ok(doc) = result {
                if let Ok(trigger) = doc.get_str("trigger") {
                    triggers.insert(trigger.to_string());
                }
            }
        }
        Ok(triggers)
    }

    /// L2/Hot: Get specific filter content with automatic hot promotion.
    pub async fn get_filter(&self, chat_id: i64, trigger: &str) -> Result<Option<DbFilter>> {
        let key = (chat_id, trigger.to_lowercase());

        // Check Hot Cache first (fastest)
        if let Some(filter) = self.hot_cache.get(&key) {
            debug!("Filter '{}' served from HOT cache", trigger);
            return Ok(Some(filter));
        }

        // Check L2 Cache
        if let Some(filter) = self.filter_cache.get(&key) {
            // Increment hit counter and potentially promote
            self.record_hit_and_maybe_promote(&key, &filter);
            return Ok(Some(filter));
        }

        // Fetch from DB
        let filter_doc = doc! {
            "chat_id": chat_id,
            "trigger": trigger.to_lowercase()
        };

        let result = self.collection.find_one(filter_doc).await?;

        if let Some(f) = &result {
            // Insert into L2 cache
            self.filter_cache.insert(key.clone(), f.clone());
            // Reset hit counter for this key
            self.hit_counter.insert(key, AtomicU64::new(1));
        }

        Ok(result)
    }

    /// Record hit and promote to hot cache if threshold reached.
    fn record_hit_and_maybe_promote(&self, key: &(i64, String), filter: &DbFilter) {
        let counter = self.hit_counter
            .entry(key.clone())
            .or_insert_with(|| AtomicU64::new(0));

        let hits = counter.fetch_add(1, Ordering::Relaxed) + 1;

        if hits >= HOT_PROMOTION_THRESHOLD {
            // Promote to hot cache
            self.hot_cache.insert(key.clone(), filter.clone());
            debug!(
                "Filter '{}' promoted to HOT cache (hits: {})",
                key.1, hits
            );
        }
    }

    /// Save a filter.
    pub async fn save_filter(&self, filter: &DbFilter) -> Result<()> {
        let filter_doc = doc! {
            "chat_id": filter.chat_id,
            "trigger": &filter.trigger
        };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter_doc, filter)
            .with_options(options)
            .await?;

        let key = (filter.chat_id, filter.trigger.clone());

        // Update both L2 and Hot caches
        self.filter_cache.insert(key.clone(), filter.clone());
        // If was in hot cache, update it too
        if self.hot_cache.get(&key).is_some() {
            self.hot_cache.insert(key.clone(), filter.clone());
        }

        // Invalidate L1 (trigger list may have changed)
        self.triggers_cache.invalidate(&filter.chat_id);

        Ok(())
    }

    /// Delete a filter.
    pub async fn delete_filter(&self, chat_id: i64, trigger: &str) -> Result<bool> {
        let filter_doc = doc! {
            "chat_id": chat_id,
            "trigger": trigger.to_lowercase()
        };

        let result = self.collection.delete_one(filter_doc).await?;

        if result.deleted_count > 0 {
            let key = (chat_id, trigger.to_lowercase());
            // Remove from all caches
            self.filter_cache.invalidate(&key);
            self.hot_cache.invalidate(&key);
            self.hit_counter.remove(&key);
            self.triggers_cache.invalidate(&chat_id);
            return Ok(true);
        }

        Ok(false)
    }

    /// Get cache statistics for monitoring.
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            hot_items: self.hit_counter.iter()
                .filter(|e| e.value().load(Ordering::Relaxed) >= HOT_PROMOTION_THRESHOLD)
                .count(),
            tracked_items: self.hit_counter.len(),
        }
    }
}

/// Cache statistics for monitoring.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hot_items: usize,
    pub tracked_items: usize,
}

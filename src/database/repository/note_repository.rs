//! Note repository with tiered caching.
//!
//! Implements L1 (Names), L2 (Content), and L2-Hot (Frequently Accessed) caching.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::Result;
use dashmap::DashMap;
use futures::StreamExt;
use mongodb::bson::{doc, Document};
use mongodb::Collection;
use tracing::debug;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use crate::database::models::DbNote;
use crate::database::Database;

/// Threshold for promoting to hot cache (access count).
const HOT_PROMOTION_THRESHOLD: u64 = 3;

/// Repository for notes with hot cache tier.
pub struct NoteRepository {
    collection: Collection<DbNote>,
    /// L1 Cache: ChatID -> List of Note Names (1 hour TTL)
    names_cache: TypedCache<i64, Vec<String>>,
    /// L2 Cache: (ChatID, Name) -> Note Content (1 min TTL)
    note_cache: TypedCache<(i64, String), DbNote>,
    /// L2-Hot Cache: (ChatID, Name) -> Note Content (10 min TTL, promoted items)
    hot_cache: TypedCache<(i64, String), DbNote>,
    /// Hit counter for promotion decisions
    hit_counter: DashMap<(i64, String), AtomicU64>,
}

impl NoteRepository {
    pub fn new(db: &Database, cache: &CacheRegistry) -> Self {
        let names_cache = cache.get_or_create(
            "note_names",
            CacheConfig::default()
                .ttl(Duration::from_secs(3600)), // 1 hour
        );

        let note_cache = cache.get_or_create(
            "note_content",
            CacheConfig::hot_data(), // 1 min TTL
        );

        // Hot cache: Longer TTL for frequently accessed notes
        let hot_cache = cache.get_or_create(
            "note_hot",
            CacheConfig::hot_promoted() // 10 min TTL, idle timeout
                .max_capacity(2_000),
        );

        Self {
            collection: db.collection("notes"),
            names_cache,
            note_cache,
            hot_cache,
            hit_counter: DashMap::with_capacity(1_000),
        }
    }

    /// L1: Get all note names for a chat.
    pub async fn get_names(&self, chat_id: i64) -> Result<Vec<String>> {
        if let Some(names) = self.names_cache.get(&chat_id) {
            return Ok(names);
        }

        let raw_coll: Collection<Document> = self.collection.clone_with_type();
        let filter = doc! { "chat_id": chat_id };
        let options = mongodb::options::FindOptions::builder()
            .projection(doc! { "name": 1, "_id": 0 })
            .sort(doc! { "name": 1 })
            .build();

        let mut cursor = raw_coll.find(filter).with_options(options).await?;
        let mut names = Vec::new();

        while let Some(result) = cursor.next().await {
            if let Ok(doc) = result {
                if let Ok(name) = doc.get_str("name") {
                    names.push(name.to_string());
                }
            }
        }

        self.names_cache.insert(chat_id, names.clone());
        Ok(names)
    }

    /// L2/Hot: Get specific note content with automatic hot promotion.
    pub async fn get_note(&self, chat_id: i64, name: &str) -> Result<Option<DbNote>> {
        let key = (chat_id, name.to_lowercase());

        // Check Hot Cache first (fastest)
        if let Some(note) = self.hot_cache.get(&key) {
            debug!("Note '{}' served from HOT cache", name);
            return Ok(Some(note));
        }

        // Check L2 Cache
        if let Some(note) = self.note_cache.get(&key) {
            // Increment hit counter and potentially promote
            self.record_hit_and_maybe_promote(&key, &note);
            return Ok(Some(note));
        }

        // Fetch from DB
        let filter = doc! {
            "chat_id": chat_id,
            "name": name.to_lowercase()
        };

        let result = self.collection.find_one(filter).await?;

        if let Some(n) = &result {
            // Insert into L2 cache
            self.note_cache.insert(key.clone(), n.clone());
            // Reset hit counter for this key
            self.hit_counter.insert(key, AtomicU64::new(1));
        }

        Ok(result)
    }

    /// Record hit and promote to hot cache if threshold reached.
    fn record_hit_and_maybe_promote(&self, key: &(i64, String), note: &DbNote) {
        let counter = self.hit_counter
            .entry(key.clone())
            .or_insert_with(|| AtomicU64::new(0));

        let hits = counter.fetch_add(1, Ordering::Relaxed) + 1;

        if hits >= HOT_PROMOTION_THRESHOLD {
            // Promote to hot cache
            self.hot_cache.insert(key.clone(), note.clone());
            debug!(
                "Note '{}' promoted to HOT cache (hits: {})",
                key.1, hits
            );
        }
    }

    /// Save a note.
    pub async fn save_note(&self, note: &DbNote) -> Result<()> {
        let filter = doc! {
            "chat_id": note.chat_id,
            "name": &note.name
        };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter, note)
            .with_options(options)
            .await?;

        let key = (note.chat_id, note.name.clone());

        // Update both L2 and Hot caches
        self.note_cache.insert(key.clone(), note.clone());
        if self.hot_cache.get(&key).is_some() {
            self.hot_cache.insert(key.clone(), note.clone());
        }

        // Invalidate L1
        self.names_cache.invalidate(&note.chat_id);

        Ok(())
    }

    /// Delete a note.
    pub async fn delete_note(&self, chat_id: i64, name: &str) -> Result<bool> {
        let filter = doc! {
            "chat_id": chat_id,
            "name": name.to_lowercase()
        };

        let result = self.collection.delete_one(filter).await?;

        if result.deleted_count > 0 {
            let key = (chat_id, name.to_lowercase());
            // Remove from all caches
            self.note_cache.invalidate(&key);
            self.hot_cache.invalidate(&key);
            self.hit_counter.remove(&key);
            self.names_cache.invalidate(&chat_id);
            return Ok(true);
        }

        Ok(false)
    }

    /// Delete all notes for a chat.
    pub async fn delete_all(&self, chat_id: i64) -> Result<u64> {
        // Get all names first to clear hit counters
        if let Ok(names) = self.get_names(chat_id).await {
            for name in names {
                let key = (chat_id, name.to_lowercase());
                self.note_cache.invalidate(&key);
                self.hot_cache.invalidate(&key);
                self.hit_counter.remove(&key);
            }
        }

        let filter = doc! { "chat_id": chat_id };
        let result = self.collection.delete_many(filter).await?;

        self.names_cache.invalidate(&chat_id);

        Ok(result.deleted_count)
    }

    /// Get cache statistics for monitoring.
    pub fn cache_stats(&self) -> NotesCacheStats {
        NotesCacheStats {
            hot_items: self.hit_counter.iter()
                .filter(|e| e.value().load(Ordering::Relaxed) >= HOT_PROMOTION_THRESHOLD)
                .count(),
            tracked_items: self.hit_counter.len(),
        }
    }
}

/// Cache statistics for monitoring.
#[derive(Debug, Clone)]
pub struct NotesCacheStats {
    pub hot_items: usize,
    pub tracked_items: usize,
}

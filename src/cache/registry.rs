//! Cache registry - Central management for all caches.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

use tracing::{debug, info};

use super::{CacheConfig, TypedCache};

/// Central registry for managing multiple typed caches.
///
/// The registry allows plugins and events to create and access
/// their own caches by name, providing isolation and easy management.
///
/// ## Example
///
/// ```rust
/// let registry = CacheRegistry::new();
///
/// // Create a cache for user data
/// let users: TypedCache<i64, User> = registry.create("users", CacheConfig::default());
///
/// // Later, retrieve the same cache
/// let users: TypedCache<i64, User> = registry.get("users").unwrap();
/// ```
#[derive(Clone)]
pub struct CacheRegistry {
    caches: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

/// Internal cache entry storing type-erased cache.
struct CacheEntry {
    cache: Box<dyn Any + Send + Sync>,
    type_id: TypeId,
    type_name: &'static str,
}

impl CacheRegistry {
    /// Create a new empty cache registry.
    pub fn new() -> Self {
        info!("Cache registry initialized");
        Self {
            caches: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new typed cache and register it.
    ///
    /// # Panics
    /// Panics if a cache with the same name but different types already exists.
    pub fn create<K, V>(&self, name: &str, config: CacheConfig) -> TypedCache<K, V>
    where
        K: Hash + Eq + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        let cache = TypedCache::new(name, config);

        let mut caches = self.caches.write().unwrap();

        if let Some(existing) = caches.get(name) {
            let expected_type = TypeId::of::<TypedCache<K, V>>();
            if existing.type_id != expected_type {
                panic!(
                    "Cache '{}' already exists with different types: expected {}, got {}",
                    name,
                    std::any::type_name::<TypedCache<K, V>>(),
                    existing.type_name
                );
            }
            // Return existing cache if types match
            return existing
                .cache
                .downcast_ref::<TypedCache<K, V>>()
                .unwrap()
                .clone();
        }

        debug!("Creating cache: {}", name);

        caches.insert(
            name.to_string(),
            CacheEntry {
                cache: Box::new(cache.clone()),
                type_id: TypeId::of::<TypedCache<K, V>>(),
                type_name: std::any::type_name::<TypedCache<K, V>>(),
            },
        );

        cache
    }

    /// Get an existing cache by name.
    ///
    /// Returns `None` if the cache doesn't exist.
    /// Returns `Some(cache)` if found and types match.
    ///
    /// # Panics
    /// Panics if the cache exists but with different types.
    pub fn get<K, V>(&self, name: &str) -> Option<TypedCache<K, V>>
    where
        K: Hash + Eq + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        let caches = self.caches.read().unwrap();

        caches.get(name).map(|entry| {
            let expected_type = TypeId::of::<TypedCache<K, V>>();
            if entry.type_id != expected_type {
                panic!(
                    "Cache '{}' type mismatch: expected {}, got {}",
                    name,
                    std::any::type_name::<TypedCache<K, V>>(),
                    entry.type_name
                );
            }
            entry
                .cache
                .downcast_ref::<TypedCache<K, V>>()
                .unwrap()
                .clone()
        })
    }

    /// Get an existing cache or create a new one if it doesn't exist.
    ///
    /// This is the recommended way to access caches in plugins/events.
    pub fn get_or_create<K, V>(&self, name: &str, config: CacheConfig) -> TypedCache<K, V>
    where
        K: Hash + Eq + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        if let Some(cache) = self.get(name) {
            return cache;
        }
        self.create(name, config)
    }

    /// Check if a cache with the given name exists.
    #[allow(dead_code)]
    pub fn contains(&self, name: &str) -> bool {
        self.caches.read().unwrap().contains_key(name)
    }

    /// Remove a cache from the registry.
    ///
    /// Returns `true` if the cache was removed.
    #[allow(dead_code)]
    pub fn remove(&self, name: &str) -> bool {
        let removed = self.caches.write().unwrap().remove(name).is_some();
        if removed {
            debug!("Removed cache: {}", name);
        }
        removed
    }

    /// Get the number of registered caches.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.caches.read().unwrap().len()
    }

    /// Check if the registry is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.caches.read().unwrap().is_empty()
    }

    /// Get a list of all registered cache names.
    #[allow(dead_code)]
    pub fn cache_names(&self) -> Vec<String> {
        self.caches.read().unwrap().keys().cloned().collect()
    }
}

impl Default for CacheRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CacheRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let caches = self.caches.read().unwrap();
        f.debug_struct("CacheRegistry")
            .field("cache_count", &caches.len())
            .field("cache_names", &caches.keys().collect::<Vec<_>>())
            .finish()
    }
}

//! Typed cache wrapper around Moka.

use std::hash::Hash;
use std::sync::Arc;

use moka::sync::Cache;

use super::CacheConfig;

/// A typed cache wrapper that provides a clean API over Moka.
///
/// This cache is:
/// - Thread-safe (uses Arc internally)
/// - LRU-based with optional TTL/TTI
/// - Clone-friendly (cloning is cheap, shares the same underlying cache)
pub struct TypedCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    inner: Arc<Cache<K, V>>,
    name: Arc<str>,
}

// Manual Clone implementation that doesn't require K: Clone, V: Clone
impl<K, V> Clone for TypedCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            name: Arc::clone(&self.name),
        }
    }
}

impl<K, V> TypedCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new typed cache with the given name and config.
    pub fn new(name: impl Into<Arc<str>>, config: CacheConfig) -> Self {
        let mut builder = Cache::builder().max_capacity(config.max_capacity);

        if let Some(ttl) = config.ttl {
            builder = builder.time_to_live(ttl);
        }

        if let Some(tti) = config.tti {
            builder = builder.time_to_idle(tti);
        }

        Self {
            inner: Arc::new(builder.build()),
            name: name.into(),
        }
    }

    /// Get the name of this cache.
    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Insert a key-value pair into the cache.
    pub fn insert(&self, key: K, value: V) {
        self.inner.insert(key, value);
    }

    /// Get a value from the cache.
    ///
    /// Returns `Some(value)` if the key exists and hasn't expired.
    pub fn get(&self, key: &K) -> Option<V> {
        self.inner.get(key)
    }

    /// Check if a key exists in the cache.
    #[allow(dead_code)]
    pub fn contains(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    /// Remove a key from the cache.
    pub fn invalidate(&self, key: &K) {
        self.inner.invalidate(key);
    }

    /// Remove all entries from the cache.
    #[allow(dead_code)]
    pub fn invalidate_all(&self) {
        self.inner.invalidate_all();
    }

    /// Get the number of entries in the cache.
    ///
    /// Note: This may not be perfectly accurate due to concurrent operations.
    #[allow(dead_code)]
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    /// Get or insert a value using a closure.
    ///
    /// If the key exists, returns the cached value.
    /// Otherwise, calls the closure to compute the value, inserts it, and returns it.
    #[allow(dead_code)]
    pub fn get_or_insert_with<F>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> V,
        K: Clone,
    {
        self.inner.get_with(key, f)
    }

    /// Get or try to insert a value using a fallible closure.
    ///
    /// Returns `Ok(value)` if found or successfully computed.
    /// Returns `Err(e)` if the closure fails.
    #[allow(dead_code)]
    pub fn get_or_try_insert_with<F, E>(&self, key: K, f: F) -> Result<V, Arc<E>>
    where
        F: FnOnce() -> Result<V, E>,
        E: Send + Sync + 'static,
        K: Clone,
    {
        self.inner.try_get_with(key, f)
    }
}

impl<K, V> std::fmt::Debug for TypedCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedCache")
            .field("name", &self.name)
            .field("entry_count", &self.inner.entry_count())
            .finish()
    }
}

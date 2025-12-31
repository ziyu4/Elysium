//! Cache configuration.

use std::time::Duration;

/// Configuration for a cache instance.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache.
    pub max_capacity: u64,

    /// Time-to-live for cache entries.
    /// After this duration, entries are automatically evicted.
    pub ttl: Option<Duration>,

    /// Time-to-idle for cache entries.
    /// Entries are evicted if not accessed within this duration.
    pub tti: Option<Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            ttl: Some(Duration::from_secs(300)), // 5 minutes
            tti: None,
        }
    }
}

impl CacheConfig {
    /// Create a new cache config with the given max capacity.
    pub fn with_capacity(max_capacity: u64) -> Self {
        Self {
            max_capacity,
            ..Default::default()
        }
    }

    /// Set max capacity for cache (builder pattern).
    #[must_use]
    pub fn max_capacity(mut self, max_capacity: u64) -> Self {
        self.max_capacity = max_capacity;
        self
    }

    /// Set time-to-live for cache entries.
    #[must_use]
    pub fn ttl(mut self, duration: Duration) -> Self {
        self.ttl = Some(duration);
        self
    }

    /// Set time-to-idle for cache entries.
    #[must_use]
    pub fn tti(mut self, duration: Duration) -> Self {
        self.tti = Some(duration);
        self
    }

    /// Disable TTL (entries never expire based on time).
    #[allow(dead_code)]
    pub fn no_ttl(mut self) -> Self {
        self.ttl = None;
        self
    }

    /// Create config optimized for frequently accessed data.
    /// Higher capacity, shorter TTL.
    #[allow(dead_code)]
    pub fn hot_data() -> Self {
        Self {
            max_capacity: 50_000,
            ttl: Some(Duration::from_secs(60)), // 1 minute
            tti: Some(Duration::from_secs(30)), // 30 seconds idle
        }
    }

    /// Create config optimized for rarely changing data.
    /// Lower capacity, longer TTL.
    #[allow(dead_code)]
    pub fn cold_data() -> Self {
        Self {
            max_capacity: 5_000,
            ttl: Some(Duration::from_secs(3600)), // 1 hour
            tti: None,
        }
    }

    /// Create config for session-like data.
    /// Medium capacity, TTI-based expiration.
    #[allow(dead_code)]
    pub fn session_data() -> Self {
        Self {
            max_capacity: 20_000,
            ttl: Some(Duration::from_secs(1800)), // 30 minutes max
            tti: Some(Duration::from_secs(300)),  // 5 minutes idle
        }
    }

    /// Create config for per-message hot path.
    /// High capacity, medium TTL for things checked every message.
    pub fn message_context() -> Self {
        Self {
            max_capacity: 10_000,
            ttl: Some(Duration::from_secs(600)), // 10 minutes
            tti: None,
        }
    }

    /// Create config for lazy-loaded rare features.
    /// Low capacity, short TTL for infrequently accessed data.
    pub fn lazy_load() -> Self {
        Self {
            max_capacity: 2_000,
            ttl: Some(Duration::from_secs(300)), // 5 minutes
            tti: None,
        }
    }

    /// Create config for hot-promoted content.
    /// Short TTL with idle timeout for frequently hit items.
    pub fn hot_promoted() -> Self {
        Self {
            max_capacity: 5_000,
            ttl: Some(Duration::from_secs(120)), // 2 minutes max
            tti: Some(Duration::from_secs(60)),  // 1 minute idle
        }
    }
}


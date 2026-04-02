//! Caching layer with LRU eviction, TTL support, and Redis integration
//!
//! This module provides a high-performance caching solution with:
//! - In-memory LRU cache with configurable capacity
//! - TTL (Time-To-Live) support for cache entries
//! - Redis backend integration for distributed caching
//! - Cache statistics and hit/miss ratios
//! - Thread-safe operations with async support

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, trace};

/// Cache operation errors
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache entry not found: {0}")]
    NotFound(String),
    
    #[error("Cache serialization error: {0}")]
    SerializationError(String),
    
    #[error("Cache deserialization error: {0}")]
    DeserializationError(String),
    
    #[error("Redis connection error: {0}")]
    RedisError(String),
    
    #[error("TTL expired for key: {0}")]
    TtlExpired(String),
    
    #[error("Cache capacity exceeded")]
    CapacityExceeded,
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for cache operations
pub type CacheResult<T> = Result<T, CacheError>;

/// Cache entry with metadata for TTL tracking
#[derive(Debug, Clone)]
pub struct CacheEntry<V> {
    /// The cached value
    pub value: V,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Expiration timestamp (None means no expiration)
    pub expires_at: Option<DateTime<Utc>>,
    
    /// Last access timestamp for LRU tracking
    pub last_accessed: DateTime<Utc>,
    
    /// Access count for LFU tracking
    pub access_count: u64,
}

impl<V> CacheEntry<V> {
    /// Create a new cache entry with optional TTL
    pub fn new(value: V, ttl_seconds: Option<i64>) -> Self {
        let now = Utc::now();
        let expires_at = ttl_seconds.map(|secs| now + Duration::seconds(secs));
        
        Self {
            value,
            created_at: now,
            expires_at,
            last_accessed: now,
            access_count: 0,
        }
    }
    
    /// Check if the entry has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }
    
    /// Update access metadata (called on cache hit)
    pub fn touch(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count += 1;
    }
}

/// Eviction policy for cache entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EvictionPolicy {
    /// Least Recently Used (default)
    #[default]
    LRU,
    
    /// Least Frequently Used
    LFU,
    
    /// First In First Out
    FIFO,
}

/// In-memory cache implementation with LRU eviction
pub struct InMemoryCache<K, V> {
    /// Cache entries
    entries: HashMap<K, CacheEntry<V>>,
    
    /// Maximum number of entries
    max_capacity: usize,
    
    /// Eviction policy
    eviction_policy: EvictionPolicy,
    
    /// Default TTL in seconds (None = no expiration)
    default_ttl_seconds: Option<i64>,
    
    /// Statistics
    stats: CacheStats,
}

impl<K, V> InMemoryCache<K, V>
where
    K: Eq + Hash + Clone + Debug,
    V: Clone + Debug,
{
    /// Create a new in-memory cache with given capacity
    pub fn new(max_capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(max_capacity.min(1024)),
            max_capacity,
            eviction_policy: EvictionPolicy::default(),
            default_ttl_seconds: None,
            stats: CacheStats::default(),
        }
    }
    
    /// Create a new cache with custom eviction policy
    pub fn with_policy(mut self, policy: EvictionPolicy) -> Self {
        self.eviction_policy = policy;
        self
    }
    
    /// Set default TTL for new entries
    pub fn with_default_ttl(mut self, ttl_seconds: i64) -> Self {
        self.default_ttl_seconds = Some(ttl_seconds);
        self
    }
    
    /// Get an entry from the cache
    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.entries.get_mut(key) {
            if entry.is_expired() {
                self.stats.increment_miss();
                self.entries.remove(key);
                debug!("Cache entry expired for key: {:?}", key);
                return None;
            }
            
            entry.touch();
            self.stats.increment_hit();
            trace!("Cache hit for key: {:?}", key);
            return Some(entry.value.clone());
        }
        
        self.stats.increment_miss();
        trace!("Cache miss for key: {:?}", key);
        None
    }
    
    /// Insert an entry into the cache
    pub fn insert(&mut self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl_seconds)
    }
    
    /// Insert an entry with custom TTL
    pub fn insert_with_ttl(&mut self, key: K, value: V, ttl_seconds: Option<i64>) {
        // Check if we need to evict
        if !self.entries.contains_key(&key) && self.entries.len() >= self.max_capacity {
            self.evict_one();
        }
        
        let entry = CacheEntry::new(value, ttl_seconds);
        self.entries.insert(key, entry);
        self.stats.increment_writes();
    }
    
    /// Remove an entry from the cache
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.entries.remove(key).map(|e| {
            self.stats.increment_deletes();
            e.value
        })
    }
    
    /// Clear all entries from the cache
    pub fn clear(&mut self) {
        self.entries.clear();
        self.stats.reset();
    }
    
    /// Check if a key exists in the cache (without updating access time)
    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.get(key)
            .map(|e| !e.is_expired())
            .unwrap_or(false)
    }
    
    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }
    
    /// Get remaining capacity
    pub fn remaining_capacity(&self) -> usize {
        self.max_capacity.saturating_sub(self.entries.len())
    }
    
    /// Evict one entry based on the eviction policy
    fn evict_one(&mut self) {
        if let Some(key_to_remove) = self.find_eviction_candidate() {
            let removed = self.entries.remove(&key_to_remove);
            debug!(
                "Evicted entry with key {:?} using {:?} policy",
                key_to_remove, self.eviction_policy
            );
            self.stats.increment_evictions();
        }
    }
    
    /// Find the best candidate for eviction based on policy
    fn find_eviction_candidate(&self) -> Option<K> {
        match self.eviction_policy {
            EvictionPolicy::LRU => self.find_lru_candidate(),
            EvictionPolicy::LFU => self.find_lfu_candidate(),
            EvictionPolicy::FIFO => self.find_fifo_candidate(),
        }
    }
    
    /// Find least recently used entry
    fn find_lru_candidate(&self) -> Option<K> {
        self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone())
    }
    
    /// Find least frequently used entry
    fn find_lfu_candidate(&self) -> Option<K> {
        self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.access_count)
            .map(|(key, _)| key.clone())
    }
    
    /// Find first inserted entry (oldest)
    fn find_fifo_candidate(&self) -> Option<K> {
        self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone())
    }
    
    /// Remove all expired entries
    pub fn cleanup_expired(&mut self) -> usize {
        let expired_keys: Vec<K> = self.entries
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(key, _)| key.clone())
            .collect();
        
        let count = expired_keys.len();
        for key in expired_keys {
            self.entries.remove(&key);
            self.stats.increment_evictions();
        }
        
        count
    }
}

/// Thread-safe wrapper for in-memory cache
pub type ThreadSafeCache<K, V> = Arc<RwLock<InMemoryCache<K, V>>>;

/// Cache statistics
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    
    /// Number of cache misses
    pub misses: u64,
    
    /// Number of writes
    pub writes: u64,
    
    /// Number of deletes
    pub deletes: u64,
    
    /// Number of evictions
    pub evictions: u64,
}

impl CacheStats {
    /// Get hit rate as a percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
    
    /// Increment hits
    pub fn increment_hit(&mut self) {
        self.hits += 1;
    }
    
    /// Increment misses
    pub fn increment_miss(&mut self) {
        self.misses += 1;
    }
    
    /// Increment writes
    pub fn increment_writes(&mut self) {
        self.writes += 1;
    }
    
    /// Increment deletes
    pub fn increment_deletes(&mut self) {
        self.deletes += 1;
    }
    
    /// Increment evictions
    pub fn increment_evictions(&mut self) {
        self.evictions += 1;
    }
    
    /// Reset all statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Trait for cache backends
#[async_trait]
pub trait CacheBackend: Send + Sync + 'static {
    /// Get a value from the cache
    async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>>;
    
    /// Set a value in the cache
    async fn set(&self, key: &str, value: Vec<u8>, ttl_seconds: Option<i64>) -> CacheResult<()>;
    
    /// Delete a value from the cache
    async fn delete(&self, key: &str) -> CacheResult<()>;
    
    /// Check if a key exists
    async fn exists(&self, key: &str) -> CacheResult<bool>;
    
    /// Clear all entries
    async fn clear(&self) -> CacheResult<()>;
    
    /// Get cache statistics
    async fn stats(&self) -> CacheResult<CacheStats>;
}

/// Redis cache backend configuration
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis server URL
    pub url: String,
    
    /// Redis database number
    pub database: u8,
    
    /// Connection pool size
    pub pool_size: usize,
    
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    
    /// Default TTL for cache entries
    pub default_ttl_seconds: Option<i64>,
    
    /// Key prefix for all entries
    pub key_prefix: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            database: 0,
            pool_size: 10,
            connection_timeout_ms: 5000,
            default_ttl_seconds: Some(3600), // 1 hour
            key_prefix: "nebula:".to_string(),
        }
    }
}

impl RedisConfig {
    /// Create a new Redis config with custom URL
    pub fn new(url: String) -> Self {
        Self {
            url,
            ..Default::default()
        }
    }
    
    /// Set the key prefix
    pub fn with_key_prefix(mut self, prefix: String) -> Self {
        self.key_prefix = prefix;
        self
    }
    
    /// Set the default TTL
    pub fn with_default_ttl(mut self, ttl_seconds: i64) -> Self {
        self.default_ttl_seconds = Some(ttl_seconds);
        self
    }
    
    /// Set connection pool size
    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Enable in-memory caching
    pub memory_cache_enabled: bool,
    
    /// In-memory cache capacity
    pub memory_cache_capacity: usize,
    
    /// In-memory cache eviction policy
    pub eviction_policy: EvictionPolicy,
    
    /// Default TTL for in-memory cache
    pub default_ttl_seconds: Option<i64>,
    
    /// Redis configuration (if using Redis backend)
    pub redis_config: Option<RedisConfig>,
    
    /// Enable cache statistics
    pub stats_enabled: bool,
    
    /// Enable cache metrics collection
    pub metrics_enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            memory_cache_enabled: true,
            memory_cache_capacity: 10_000,
            eviction_policy: EvictionPolicy::LRU,
            default_ttl_seconds: Some(3600), // 1 hour
            redis_config: None,
            stats_enabled: true,
            metrics_enabled: true,
        }
    }
}

impl CacheConfig {
    /// Create a new cache config with memory cache only
    pub fn memory_only() -> Self {
        Self {
            memory_cache_enabled: true,
            memory_cache_capacity: 10_000,
            eviction_policy: EvictionPolicy::LRU,
            default_ttl_seconds: Some(3600),
            redis_config: None,
            stats_enabled: true,
            metrics_enabled: true,
        }
    }
    
    /// Create a new cache config with Redis backend
    pub fn with_redis(redis_config: RedisConfig) -> Self {
        Self {
            memory_cache_enabled: true,
            memory_cache_capacity: 10_000,
            eviction_policy: EvictionPolicy::LRU,
            default_ttl_seconds: redis_config.default_ttl_seconds,
            redis_config: Some(redis_config),
            stats_enabled: true,
            metrics_enabled: true,
        }
    }
    
    /// Set the in-memory cache capacity
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.memory_cache_capacity = capacity;
        self
    }
    
    /// Set the eviction policy
    pub fn with_eviction_policy(mut self, policy: EvictionPolicy) -> Self {
        self.eviction_policy = policy;
        self
    }
    
    /// Set the default TTL
    pub fn with_default_ttl(mut self, ttl_seconds: i64) -> Self {
        self.default_ttl_seconds = Some(ttl_seconds);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_entry_creation() {
        let entry = CacheEntry::new("test_value".to_string(), Some(60));
        assert_eq!(entry.value, "test_value");
        assert!(entry.expires_at.is_some());
        assert!(!entry.is_expired());
    }
    
    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("test_value".to_string(), Some(0));
        // With 0 TTL, it should be expired immediately
        assert!(entry.is_expired());
    }
    
    #[test]
    fn test_in_memory_cache_basic_operations() {
        let mut cache: InMemoryCache<String, String> = InMemoryCache::new(100);
        
        // Test insert and get
        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        
        // Test contains_key
        assert!(cache.contains_key(&"key1".to_string()));
        assert!(!cache.contains_key(&"key2".to_string()));
        
        // Test remove
        assert_eq!(cache.remove(&"key1".to_string()), Some("value1".to_string()));
        assert!(!cache.contains_key(&"key1".to_string()));
        
        // Test statistics - hits and writes are tracked
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().writes, 1);
        assert_eq!(cache.stats().deletes, 1);
    }
    
    #[test]
    fn test_cache_eviction_lru() {
        let mut cache: InMemoryCache<i32, String> = InMemoryCache::new(3)
            .with_policy(EvictionPolicy::LRU);
        
        cache.insert(1, "one".to_string());
        cache.insert(2, "two".to_string());
        cache.insert(3, "three".to_string());
        
        // Access key 1 to make it recently used
        cache.get(&1);
        
        // Insert key 4, should evict key 2 (least recently used)
        cache.insert(4, "four".to_string());
        
        assert!(cache.contains_key(&1));
        assert!(!cache.contains_key(&2));
        assert!(cache.contains_key(&3));
        assert!(cache.contains_key(&4));
        
        assert_eq!(cache.stats().evictions, 1);
    }
    
    #[test]
    fn test_cache_eviction_lfu() {
        let mut cache: InMemoryCache<i32, String> = InMemoryCache::new(3)
            .with_policy(EvictionPolicy::LFU);
        
        cache.insert(1, "one".to_string());
        cache.insert(2, "two".to_string());
        cache.insert(3, "three".to_string());
        
        // Access key 1 and 3 multiple times
        cache.get(&1);
        cache.get(&1);
        cache.get(&3);
        
        // Insert key 4, should evict key 2 (least frequently used)
        cache.insert(4, "four".to_string());
        
        assert!(cache.contains_key(&1));
        assert!(!cache.contains_key(&2));
        assert!(cache.contains_key(&3));
        assert!(cache.contains_key(&4));
    }
    
    #[test]
    fn test_cache_with_ttl() {
        let mut cache: InMemoryCache<String, String> = InMemoryCache::new(100)
            .with_default_ttl(1); // 1 second TTL
        
        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        
        // The entry should still be valid (just inserted)
        assert!(cache.contains_key(&"key1".to_string()));
    }
    
    #[test]
    fn test_cache_clear() {
        let mut cache: InMemoryCache<String, String> = InMemoryCache::new(100);
        
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        
        assert_eq!(cache.len(), 2);
        
        cache.clear();
        
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }
    
    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        stats.hits = 80;
        stats.misses = 20;
        
        assert_eq!(stats.hit_rate(), 80.0);
    }
    
    #[test]
    fn test_redis_config_builder() {
        let config = RedisConfig::new("redis://myserver:6379".to_string())
            .with_key_prefix("myapp:".to_string())
            .with_default_ttl(7200)
            .with_pool_size(20);
        
        assert_eq!(config.url, "redis://myserver:6379");
        assert_eq!(config.key_prefix, "myapp:");
        assert_eq!(config.default_ttl_seconds, Some(7200));
        assert_eq!(config.pool_size, 20);
    }
    
    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::memory_only()
            .with_capacity(5000)
            .with_eviction_policy(EvictionPolicy::FIFO)
            .with_default_ttl(1800);
        
        assert_eq!(config.memory_cache_capacity, 5000);
        assert_eq!(config.eviction_policy, EvictionPolicy::FIFO);
        assert_eq!(config.default_ttl_seconds, Some(1800));
        assert!(config.redis_config.is_none());
    }
    
    #[test]
    fn test_cleanup_expired() {
        let mut cache: InMemoryCache<String, String> = InMemoryCache::new(100);
        
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert_with_ttl("key2".to_string(), "value2".to_string(), Some(0)); // Expired
        cache.insert("key3".to_string(), "value3".to_string());
        
        let removed = cache.cleanup_expired();
        assert!(removed >= 1); // At least key2 should be removed
        
        assert!(cache.contains_key(&"key1".to_string()));
        assert!(!cache.contains_key(&"key2".to_string()));
        assert!(cache.contains_key(&"key3".to_string()));
    }
}

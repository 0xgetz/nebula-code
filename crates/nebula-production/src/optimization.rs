//! Performance optimization configuration combining cache, pool, and optimization strategies
//!
//! This module provides a unified configuration for performance optimizations:
//! - Cache configuration integration
//! - Connection pool configuration integration
//! - Load balancing strategies
//! - Performance monitoring and tuning
//! - Resource allocation and limits

use crate::cache::{CacheConfig, EvictionPolicy};
use crate::pool::PoolConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategy {
    /// Round-robin distribution (default)
    #[default]
    RoundRobin,
    
    /// Least connections - route to server with fewest active connections
    LeastConnections,
    
    /// Random selection
    Random,
    
    /// Weighted round-robin based on server capacity
    WeightedRoundRobin,
    
    /// IP hash for session persistence
    IpHash,
}

/// Optimization level preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationLevel {
    /// Default balanced settings
    #[default]
    Balanced,
    
    /// Optimized for low latency
    LowLatency,
    
    /// Optimized for high throughput
    HighThroughput,
    
    /// Optimized for low memory usage
    LowMemory,
    
    /// Custom configuration
    Custom,
}

/// Performance configuration combining all optimization strategies
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Optimization level preset
    pub optimization_level: OptimizationLevel,
    
    /// Cache configuration
    pub cache: CacheConfig,
    
    /// Connection pool configuration
    pub pool: PoolConfig,
    
    /// Load balancing strategy
    pub load_balancing: LoadBalancingStrategy,
    
    /// Enable performance metrics collection
    pub metrics_enabled: bool,
    
    /// Metrics collection interval in seconds
    pub metrics_interval_seconds: u64,
    
    /// Enable automatic tuning
    pub auto_tuning_enabled: bool,
    
    /// Auto-tuning interval in seconds
    pub auto_tuning_interval_seconds: u64,
    
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    
    /// Enable request compression
    pub compression_enabled: bool,
    
    /// Compression level (1-9)
    pub compression_level: u32,
    
    /// Enable response buffering
    pub response_buffering_enabled: bool,
    
    /// Response buffer size in bytes
    pub response_buffer_size: usize,
    
    /// Thread pool size for async operations
    pub thread_pool_size: usize,
    
    /// Enable keep-alive for connections
    pub keep_alive_enabled: bool,
    
    /// Keep-alive timeout in seconds
    pub keep_alive_timeout_seconds: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            optimization_level: OptimizationLevel::Balanced,
            cache: CacheConfig::default(),
            pool: PoolConfig::default(),
            load_balancing: LoadBalancingStrategy::default(),
            metrics_enabled: true,
            metrics_interval_seconds: 10,
            auto_tuning_enabled: false,
            auto_tuning_interval_seconds: 60,
            max_concurrent_requests: 1000,
            request_timeout_ms: 30000,
            compression_enabled: true,
            compression_level: 6,
            response_buffering_enabled: true,
            response_buffer_size: 65536, // 64KB
            thread_pool_size: std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4),
            keep_alive_enabled: true,
            keep_alive_timeout_seconds: 120,
        }
    }
}

impl PerformanceConfig {
    /// Create a new performance config with the balanced preset
    pub fn balanced() -> Self {
        Self {
            optimization_level: OptimizationLevel::Balanced,
            cache: CacheConfig::memory_only()
                .with_capacity(10_000)
                .with_eviction_policy(EvictionPolicy::LRU)
                .with_default_ttl(3600),
            pool: PoolConfig::new(5, 20)
                .with_connection_timeout(5000)
                .with_max_lifetime(3600)
                .with_max_idle_time(600),
            load_balancing: LoadBalancingStrategy::RoundRobin,
            metrics_enabled: true,
            metrics_interval_seconds: 10,
            auto_tuning_enabled: false,
            auto_tuning_interval_seconds: 60,
            max_concurrent_requests: 1000,
            request_timeout_ms: 30000,
            compression_enabled: true,
            compression_level: 6,
            response_buffering_enabled: true,
            response_buffer_size: 65536,
            thread_pool_size: std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4),
            keep_alive_enabled: true,
            keep_alive_timeout_seconds: 120,
        }
    }
    
    /// Create a config optimized for low latency
    pub fn low_latency() -> Self {
        Self {
            optimization_level: OptimizationLevel::LowLatency,
            cache: CacheConfig::memory_only()
                .with_capacity(50_000)
                .with_eviction_policy(EvictionPolicy::LRU)
                .with_default_ttl(1800),
            pool: PoolConfig::new(10, 50)
                .with_connection_timeout(2000)
                .with_max_lifetime(1800)
                .with_max_idle_time(300)
                .with_test_on_borrow(false),
            load_balancing: LoadBalancingStrategy::LeastConnections,
            metrics_enabled: true,
            metrics_interval_seconds: 5,
            auto_tuning_enabled: true,
            auto_tuning_interval_seconds: 30,
            max_concurrent_requests: 2000,
            request_timeout_ms: 10000,
            compression_enabled: false, // Compression adds latency
            compression_level: 1,
            response_buffering_enabled: false,
            response_buffer_size: 16384,
            thread_pool_size: std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4) * 2,
            keep_alive_enabled: true,
            keep_alive_timeout_seconds: 60,
        }
    }
    
    /// Create a config optimized for high throughput
    pub fn high_throughput() -> Self {
        Self {
            optimization_level: OptimizationLevel::HighThroughput,
            cache: CacheConfig::memory_only()
                .with_capacity(100_000)
                .with_eviction_policy(EvictionPolicy::LRU)
                .with_default_ttl(7200),
            pool: PoolConfig::new(20, 100)
                .with_connection_timeout(10000)
                .with_max_lifetime(7200)
                .with_max_idle_time(1200)
                .with_test_on_borrow(true),
            load_balancing: LoadBalancingStrategy::WeightedRoundRobin,
            metrics_enabled: true,
            metrics_interval_seconds: 30,
            auto_tuning_enabled: true,
            auto_tuning_interval_seconds: 120,
            max_concurrent_requests: 5000,
            request_timeout_ms: 60000,
            compression_enabled: true,
            compression_level: 9,
            response_buffering_enabled: true,
            response_buffer_size: 262144, // 256KB
            thread_pool_size: std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4) * 4,
            keep_alive_enabled: true,
            keep_alive_timeout_seconds: 300,
        }
    }
    
    /// Create a config optimized for low memory usage
    pub fn low_memory() -> Self {
        Self {
            optimization_level: OptimizationLevel::LowMemory,
            cache: CacheConfig::memory_only()
                .with_capacity(1000)
                .with_eviction_policy(EvictionPolicy::FIFO)
                .with_default_ttl(300),
            pool: PoolConfig::new(2, 10)
                .with_connection_timeout(5000)
                .with_max_lifetime(1800)
                .with_max_idle_time(120)
                .with_test_on_borrow(true),
            load_balancing: LoadBalancingStrategy::RoundRobin,
            metrics_enabled: true,
            metrics_interval_seconds: 60,
            auto_tuning_enabled: false,
            auto_tuning_interval_seconds: 300,
            max_concurrent_requests: 100,
            request_timeout_ms: 30000,
            compression_enabled: true,
            compression_level: 9,
            response_buffering_enabled: false,
            response_buffer_size: 4096,
            thread_pool_size: std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4),
            keep_alive_enabled: false,
            keep_alive_timeout_seconds: 30,
        }
    }
    
    /// Apply the optimization level settings
    pub fn with_optimization_level(self, level: OptimizationLevel) -> Self {
        match level {
            OptimizationLevel::Balanced => Self::balanced(),
            OptimizationLevel::LowLatency => Self::low_latency(),
            OptimizationLevel::HighThroughput => Self::high_throughput(),
            OptimizationLevel::LowMemory => Self::low_memory(),
            OptimizationLevel::Custom => Self {
            optimization_level: OptimizationLevel::Custom,
            ..self
        },
        }
    }
    
    /// Set the cache configuration
    pub fn with_cache_config(mut self, cache: CacheConfig) -> Self {
        self.cache = cache;
        self
    }
    
    /// Set the pool configuration
    pub fn with_pool_config(mut self, pool: PoolConfig) -> Self {
        self.pool = pool;
        self
    }
    
    /// Set the load balancing strategy
    pub fn with_load_balancing(mut self, strategy: LoadBalancingStrategy) -> Self {
        self.load_balancing = strategy;
        self
    }
    
    /// Enable or disable metrics collection
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.metrics_enabled = enabled;
        self
    }
    
    /// Set the metrics collection interval
    pub fn with_metrics_interval(mut self, interval: Duration) -> Self {
        self.metrics_interval_seconds = interval.as_secs();
        self
    }
    
    /// Enable or disable auto-tuning
    pub fn with_auto_tuning(mut self, enabled: bool) -> Self {
        self.auto_tuning_enabled = enabled;
        self
    }
    
    /// Set the maximum concurrent requests
    pub fn with_max_concurrent_requests(mut self, max: usize) -> Self {
        self.max_concurrent_requests = max;
        self
    }
    
    /// Set the request timeout
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout_ms = timeout.as_millis() as u64;
        self
    }
    
    /// Enable or disable compression
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = enabled;
        self
    }
    
    /// Set the compression level (1-9)
    pub fn with_compression_level(mut self, level: u32) -> Self {
        self.compression_level = level.clamp(1, 9);
        self
    }
    
    /// Enable or disable response buffering
    pub fn with_response_buffering(mut self, enabled: bool) -> Self {
        self.response_buffering_enabled = enabled;
        self
    }
    
    /// Set the response buffer size
    pub fn with_response_buffer_size(mut self, size: usize) -> Self {
        self.response_buffer_size = size;
        self
    }
    
    /// Set the thread pool size
    pub fn with_thread_pool_size(mut self, size: usize) -> Self {
        self.thread_pool_size = size;
        self
    }
    
    /// Enable or disable keep-alive
    pub fn with_keep_alive(mut self, enabled: bool) -> Self {
        self.keep_alive_enabled = enabled;
        self
    }
    
    /// Set the keep-alive timeout
    pub fn with_keep_alive_timeout(mut self, timeout: Duration) -> Self {
        self.keep_alive_timeout_seconds = timeout.as_secs();
        self
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        self.pool.validate().map_err(|e| e.to_string())?;
        
        if self.compression_level < 1 || self.compression_level > 9 {
            return Err("Compression level must be between 1 and 9".to_string());
        }
        
        if self.thread_pool_size == 0 {
            return Err("Thread pool size must be greater than 0".to_string());
        }
        
        if self.max_concurrent_requests == 0 {
            return Err("Max concurrent requests must be greater than 0".to_string());
        }
        
        Ok(())
    }
    
    /// Log the current configuration
    pub fn log_configuration(&self) {
        info!("Performance Configuration:");
        info!("  Optimization Level: {:?}", self.optimization_level);
        info!("  Cache:");
        info!("    Memory Cache Enabled: {}", self.cache.memory_cache_enabled);
        info!("    Capacity: {}", self.cache.memory_cache_capacity);
        info!("    Eviction Policy: {:?}", self.cache.eviction_policy);
        info!("    Default TTL: {:?} seconds", self.cache.default_ttl_seconds);
        info!("  Connection Pool:");
        info!("    Min Connections: {}", self.pool.min_connections);
        info!("    Max Connections: {}", self.pool.max_connections);
        info!("    Connection Timeout: {}ms", self.pool.connection_timeout_ms);
        info!("    Max Lifetime: {}s", self.pool.max_lifetime_seconds);
        info!("    Max Idle Time: {}s", self.pool.max_idle_time_seconds);
        info!("  Load Balancing: {:?}", self.load_balancing);
        info!("  Metrics Enabled: {}", self.metrics_enabled);
        info!("  Metrics Interval: {}s", self.metrics_interval_seconds);
        info!("  Auto Tuning: {}", self.auto_tuning_enabled);
        info!("  Max Concurrent Requests: {}", self.max_concurrent_requests);
        info!("  Request Timeout: {}ms", self.request_timeout_ms);
        info!("  Compression: {} (level {})", self.compression_enabled, self.compression_level);
        info!("  Response Buffering: {}", self.response_buffering_enabled);
        info!("  Response Buffer Size: {} bytes", self.response_buffer_size);
        info!("  Thread Pool Size: {}", self.thread_pool_size);
        info!("  Keep-Alive: {} (timeout {}s)", self.keep_alive_enabled, self.keep_alive_timeout_seconds);
    }
}

/// Performance metrics for monitoring
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    
    /// Requests per second
    pub requests_per_second: f64,
    
    /// Cache hit rate percentage
    pub cache_hit_rate: f64,
    
    /// Pool utilization percentage
    pub pool_utilization: f64,
    
    /// Error rate percentage
    pub error_rate: f64,
    
    /// Active connections count
    pub active_connections: usize,
    
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
}

impl PerformanceMetrics {
    /// Create new metrics with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Check if any metric indicates poor performance
    pub fn is_performance_degraded(&self) -> bool {
        self.avg_response_time_ms > 1000.0 || // > 1 second
        self.error_rate > 5.0 || // > 5% errors
        self.pool_utilization > 90.0 || // Pool nearly exhausted
        self.cpu_usage_percent > 85.0 // High CPU usage
    }
    
    /// Get performance health status
    pub fn health_status(&self) -> PerformanceHealth {
        if self.is_performance_degraded() {
            PerformanceHealth::Degraded
        } else if self.avg_response_time_ms > 500.0 || self.error_rate > 1.0 {
            PerformanceHealth::Warning
        } else {
            PerformanceHealth::Healthy
        }
    }
}

/// Performance health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceHealth {
    /// All metrics within acceptable ranges
    Healthy,
    
    /// Some metrics approaching thresholds
    Warning,
    
    /// Performance is degraded
    Degraded,
}

/// Helper function to get the number of CPUs
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_performance_config_default() {
        let config = PerformanceConfig::default();
        assert_eq!(config.optimization_level, OptimizationLevel::Balanced);
        assert!(config.metrics_enabled);
        assert!(config.compression_enabled);
        assert!(config.keep_alive_enabled);
    }
    
    #[test]
    fn test_performance_config_presets() {
        let balanced = PerformanceConfig::balanced();
        assert_eq!(balanced.optimization_level, OptimizationLevel::Balanced);
        assert_eq!(balanced.cache.memory_cache_capacity, 10_000);
        assert_eq!(balanced.pool.max_connections, 20);
        
        let low_latency = PerformanceConfig::low_latency();
        assert_eq!(low_latency.optimization_level, OptimizationLevel::LowLatency);
        assert_eq!(low_latency.cache.memory_cache_capacity, 50_000);
        assert_eq!(low_latency.pool.max_connections, 50);
        assert!(!low_latency.compression_enabled);
        
        let high_throughput = PerformanceConfig::high_throughput();
        assert_eq!(high_throughput.optimization_level, OptimizationLevel::HighThroughput);
        assert_eq!(high_throughput.cache.memory_cache_capacity, 100_000);
        assert_eq!(high_throughput.pool.max_connections, 100);
        assert!(high_throughput.compression_enabled);
        
        let low_memory = PerformanceConfig::low_memory();
        assert_eq!(low_memory.optimization_level, OptimizationLevel::LowMemory);
        assert_eq!(low_memory.cache.memory_cache_capacity, 1000);
        assert_eq!(low_memory.pool.max_connections, 10);
        assert!(!low_memory.keep_alive_enabled);
    }
    
    #[test]
    fn test_performance_config_builder() {
        let config = PerformanceConfig::balanced()
            .with_cache_config(CacheConfig::memory_only().with_capacity(5000))
            .with_pool_config(PoolConfig::new(10, 30))
            .with_load_balancing(LoadBalancingStrategy::LeastConnections)
            .with_max_concurrent_requests(2000)
            .with_request_timeout(Duration::from_secs(15))
            .with_compression(false)
            .with_keep_alive(false);
        
        // Verify builder methods work - config is still balanced
        assert_eq!(config.optimization_level, OptimizationLevel::Balanced);
        assert_eq!(config.cache.memory_cache_capacity, 5000);
        assert_eq!(config.pool.min_connections, 10);
        assert_eq!(config.pool.max_connections, 30);
        assert_eq!(config.load_balancing, LoadBalancingStrategy::LeastConnections);
        assert_eq!(config.max_concurrent_requests, 2000);
        assert_eq!(config.request_timeout_ms, 15000);
        assert!(!config.compression_enabled);
        assert!(!config.keep_alive_enabled);
    }
    
    #[test]
    fn test_performance_config_validation() {
        let config = PerformanceConfig::balanced();
        assert!(config.validate().is_ok());
        
        let mut invalid_config = PerformanceConfig::balanced();
        invalid_config.compression_level = 10; // Invalid
        assert!(invalid_config.validate().is_err());
        
        let mut invalid_config = PerformanceConfig::balanced();
        invalid_config.thread_pool_size = 0;
        assert!(invalid_config.validate().is_err());
    }
    
    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics {
            avg_response_time_ms: 250.0,
            requests_per_second: 1000.0,
            cache_hit_rate: 85.0,
            pool_utilization: 45.0,
            error_rate: 0.5,
            active_connections: 15,
            memory_usage_bytes: 1024 * 1024 * 512, // 512MB
            cpu_usage_percent: 35.0,
        };
        
        assert!(!metrics.is_performance_degraded());
        assert_eq!(metrics.health_status(), PerformanceHealth::Healthy);
        
        let degraded_metrics = PerformanceMetrics {
            avg_response_time_ms: 1500.0,
            requests_per_second: 100.0,
            cache_hit_rate: 50.0,
            pool_utilization: 95.0,
            error_rate: 10.0,
            active_connections: 50,
            memory_usage_bytes: 1024 * 1024 * 1024, // 1GB
            cpu_usage_percent: 90.0,
        };
        
        assert!(degraded_metrics.is_performance_degraded());
        assert_eq!(degraded_metrics.health_status(), PerformanceHealth::Degraded);
    }
    
    #[test]
    fn test_load_balancing_strategies() {
        let strategies = [
            LoadBalancingStrategy::RoundRobin,
            LoadBalancingStrategy::LeastConnections,
            LoadBalancingStrategy::Random,
            LoadBalancingStrategy::WeightedRoundRobin,
            LoadBalancingStrategy::IpHash,
        ];
        
        for strategy in strategies {
            let config = PerformanceConfig::balanced()
                .with_load_balancing(strategy);
            assert_eq!(config.load_balancing, strategy);
        }
    }
    
    #[test]
    fn test_optimization_levels() {
        // Test that each preset creates a valid config
        let balanced = PerformanceConfig::balanced();
        assert_eq!(balanced.optimization_level, OptimizationLevel::Balanced);
        
        let low_latency = PerformanceConfig::low_latency();
        assert_eq!(low_latency.optimization_level, OptimizationLevel::LowLatency);
        
        let high_throughput = PerformanceConfig::high_throughput();
        assert_eq!(high_throughput.optimization_level, OptimizationLevel::HighThroughput);
        
        let low_memory = PerformanceConfig::low_memory();
        assert_eq!(low_memory.optimization_level, OptimizationLevel::LowMemory);
        
        // Custom level can be set directly on a default config
        let custom = PerformanceConfig::default().with_optimization_level(OptimizationLevel::Custom);
        assert_eq!(custom.optimization_level, OptimizationLevel::Custom);
    }
}

//! Rate limiting with sliding window algorithm.
//!
//! Provides per-client rate limiting to protect against abuse and ensure fair usage.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors that can occur in rate limiting operations.
#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded for client {0}")]
    LimitExceeded(String),
    #[error("Invalid rate limit configuration: {0}")]
    InvalidConfig(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Rate limit result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in the current window
    pub remaining: u64,
    /// Total limit for the window
    pub limit: u64,
    /// Time until the window resets (in seconds)
    pub reset_seconds: u64,
    /// Retry-after header value (if limited)
    pub retry_after: Option<u64>,
}

/// Sliding window rate limiter entry for a single client.
#[derive(Debug, Clone)]
struct WindowEntry {
    /// Timestamps of requests in the current window
    timestamps: Vec<Instant>,
    /// Window size
    window_size: Duration,
    /// Maximum requests allowed in the window
    max_requests: u64,
}

impl WindowEntry {
    fn new(window_size: Duration, max_requests: u64) -> Self {
        Self {
            timestamps: Vec::with_capacity(max_requests as usize),
            window_size,
            max_requests,
        }
    }

    /// Check if a request is allowed and record it if so.
    fn check_and_record(&mut self, now: Instant) -> RateLimitResult {
        let window_start = now - self.window_size;

        // Remove expired timestamps (outside the current window)
        self.timestamps.retain(|&t| t > window_start);

        let current_count = self.timestamps.len();

        if current_count < self.max_requests as usize {
            // Allow the request
            self.timestamps.push(now);
            let remaining = (self.max_requests as usize - self.timestamps.len()) as u64;

            // Calculate reset time based on oldest timestamp in window
            let reset_seconds = if let Some(&oldest) = self.timestamps.first() {
                let expires = oldest + self.window_size;
                if expires > now {
                    (expires - now).as_secs().max(1)
                } else {
                    self.window_size.as_secs()
                }
            } else {
                self.window_size.as_secs()
            };

            RateLimitResult {
                allowed: true,
                remaining,
                limit: self.max_requests,
                reset_seconds,
                retry_after: None,
            }
        } else {
            // Deny the request
            let oldest = self.timestamps.first().unwrap();
            let reset_seconds = ((*oldest + self.window_size) - now).as_secs().max(1);

            RateLimitResult {
                allowed: false,
                remaining: 0,
                limit: self.max_requests,
                reset_seconds,
                retry_after: Some(reset_seconds),
            }
        }
    }
}

/// Rate limit configuration for a specific tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitTier {
    /// Tier name (e.g., "default", "premium", "admin")
    pub name: String,
    /// Maximum requests allowed in the window
    pub max_requests: u64,
    /// Window size in seconds
    pub window_seconds: u64,
    /// Burst capacity (allows short bursts above the limit)
    pub burst_capacity: Option<u64>,
}

impl Default for RateLimitTier {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            max_requests: 100,
            window_seconds: 60,
            burst_capacity: None,
        }
    }
}

/// Rate limiter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterConfig {
    /// Default tier for unauthenticated clients
    pub default_tier: RateLimitTier,
    /// All available tiers
    pub tiers: Vec<RateLimitTier>,
    /// Enable rate limiting
    pub enabled: bool,
    /// Key extraction strategy (header, IP, API key)
    pub key_strategy: RateLimitKeyStrategy,
    /// Bypass rate limiting for these client IDs
    pub bypass_clients: Vec<String>,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            default_tier: RateLimitTier::default(),
            tiers: vec![
                RateLimitTier {
                    name: "default".to_string(),
                    max_requests: 100,
                    window_seconds: 60,
                    burst_capacity: Some(20),
                },
                RateLimitTier {
                    name: "premium".to_string(),
                    max_requests: 1000,
                    window_seconds: 60,
                    burst_capacity: Some(100),
                },
                RateLimitTier {
                    name: "admin".to_string(),
                    max_requests: 10000,
                    window_seconds: 60,
                    burst_capacity: Some(1000),
                },
            ],
            enabled: true,
            key_strategy: RateLimitKeyStrategy::Header("X-Client-ID".to_string()),
            bypass_clients: Vec::new(),
        }
    }
}

/// Strategy for extracting the rate limit key from a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitKeyStrategy {
    /// Extract from a specific header
    Header(String),
    /// Use client IP address
    IpAddress,
    /// Use API key
    ApiKey,
    /// Custom key
    Custom(String),
}

/// Sliding window rate limiter with per-client tracking.
pub struct RateLimiter {
    /// Rate limit entries per client
    entries: Arc<RwLock<HashMap<String, WindowEntry>>>,
    /// Configuration
    config: RateLimiterConfig,
    /// Tier lookup by name
    tier_map: HashMap<String, RateLimitTier>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: RateLimiterConfig) -> Result<Self, RateLimitError> {
        if !config.enabled {
            return Ok(Self {
                entries: Arc::new(RwLock::new(HashMap::new())),
                config,
                tier_map: HashMap::new(),
            });
        }

        // Validate configuration
        if config.default_tier.max_requests == 0 {
            return Err(RateLimitError::InvalidConfig(
                "Default tier max_requests must be > 0".to_string(),
            ));
        }

        if config.default_tier.window_seconds == 0 {
            return Err(RateLimitError::InvalidConfig(
                "Default tier window_seconds must be > 0".to_string(),
            ));
        }

        // Build tier map
        let mut tier_map = HashMap::new();
        tier_map.insert(config.default_tier.name.clone(), config.default_tier.clone());
        for tier in &config.tiers {
            if tier.max_requests == 0 || tier.window_seconds == 0 {
                return Err(RateLimitError::InvalidConfig(format!(
                    "Tier {} has invalid max_requests or window_seconds",
                    tier.name
                )));
            }
            tier_map.insert(tier.name.clone(), tier.clone());
        }

        Ok(Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            config,
            tier_map,
        })
    }

    /// Check if a request from the given client is allowed.
    pub async fn check(&self, client_id: &str, tier_name: Option<&str>) -> RateLimitResult {
        if !self.config.enabled {
            return RateLimitResult {
                allowed: true,
                remaining: u64::MAX,
                limit: u64::MAX,
                reset_seconds: 0,
                retry_after: None,
            };
        }

        // Check bypass list
        if self.config.bypass_clients.contains(&client_id.to_string()) {
            return RateLimitResult {
                allowed: true,
                remaining: u64::MAX,
                limit: u64::MAX,
                reset_seconds: 0,
                retry_after: None,
            };
        }

        // Get tier configuration
        let tier = tier_name
            .and_then(|name| self.tier_map.get(name))
            .unwrap_or(&self.config.default_tier);

        let window_size = Duration::from_secs(tier.window_seconds);
        let now = Instant::now();

        // Get or create entry for this client
        let mut entries = self.entries.write().await;
        let entry = entries
            .entry(client_id.to_string())
            .or_insert_with(|| WindowEntry::new(window_size, tier.max_requests));

        entry.check_and_record(now)
    }

    /// Check rate limit for a request with headers.
    pub async fn check_request(
        &self,
        client_id: &str,
        tier: Option<&str>,
    ) -> RateLimitResult {
        self.check(client_id, tier).await
    }

    /// Get current rate limit status for a client without recording a request.
    pub async fn status(&self, client_id: &str, tier_name: Option<&str>) -> RateLimitResult {
        if !self.config.enabled {
            return RateLimitResult {
                allowed: true,
                remaining: u64::MAX,
                limit: u64::MAX,
                reset_seconds: 0,
                retry_after: None,
            };
        }

        let tier = tier_name
            .and_then(|name| self.tier_map.get(name))
            .unwrap_or(&self.config.default_tier);

        let window_size = Duration::from_secs(tier.window_seconds);
        let now = Instant::now();

        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(client_id) {
            let window_start = now - window_size;
            let active_count = entry.timestamps.iter().filter(|&&t| t > window_start).count();
            let remaining = (tier.max_requests as usize).saturating_sub(active_count) as u64;

            let reset_seconds = if let Some(&oldest) = entry.timestamps.iter().filter(|&&t| t > window_start).next() {
                let expires = oldest + window_size;
                if expires > now {
                    (expires - now).as_secs().max(1)
                } else {
                    window_size.as_secs()
                }
            } else {
                window_size.as_secs()
            };

            RateLimitResult {
                allowed: remaining > 0,
                remaining,
                limit: tier.max_requests,
                reset_seconds,
                retry_after: if remaining == 0 { Some(reset_seconds) } else { None },
            }
        } else {
            RateLimitResult {
                allowed: true,
                remaining: tier.max_requests,
                limit: tier.max_requests,
                reset_seconds: window_size.as_secs(),
                retry_after: None,
            }
        }
    }

    /// Reset rate limit for a specific client.
    pub async fn reset(&self, client_id: &str) {
        let mut entries = self.entries.write().await;
        entries.remove(client_id);
    }

    /// Reset all rate limits.
    pub async fn reset_all(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Get the configuration.
    pub fn config(&self) -> &RateLimiterConfig {
        &self.config
    }
}

/// Middleware layer for rate limiting (for use with Tower/Axum).
#[derive(Clone)]
pub struct RateLimitMiddleware {
    limiter: Arc<RateLimiter>,
}

impl RateLimitMiddleware {
    /// Create a new rate limit middleware.
    pub fn new(limiter: Arc<RateLimiter>) -> Self {
        Self { limiter }
    }

    /// Get the underlying rate limiter.
    pub fn limiter(&self) -> &Arc<RateLimiter> {
        &self.limiter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let config = RateLimiterConfig {
            enabled: true,
            default_tier: RateLimitTier {
                name: "test".to_string(),
                max_requests: 5,
                window_seconds: 60,
                burst_capacity: None,
            },
            tiers: Vec::new(),
            key_strategy: RateLimitKeyStrategy::Header("X-Client-ID".to_string()),
            bypass_clients: Vec::new(),
        };

        let limiter = RateLimiter::new(config).unwrap();

        // First 5 requests should be allowed
        for i in 1..=5 {
            let result = limiter.check("client1", None).await;
            assert!(result.allowed, "Request {} should be allowed", i);
            assert_eq!(result.remaining, 5 - i);
        }

        // 6th request should be denied
        let result = limiter.check("client1", None).await;
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
        assert!(result.retry_after.is_some());
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let config = RateLimiterConfig {
            enabled: false,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config).unwrap();
        let result = limiter.check("client1", None).await;
        assert!(result.allowed);
        assert_eq!(result.remaining, u64::MAX);
    }

    #[tokio::test]
    async fn test_rate_limiter_bypass() {
        let config = RateLimiterConfig {
            enabled: true,
            default_tier: RateLimitTier {
                name: "test".to_string(),
                max_requests: 1,
                window_seconds: 60,
                burst_capacity: None,
            },
            tiers: Vec::new(),
            bypass_clients: vec!["admin".to_string()],
            ..Default::default()
        };

        let limiter = RateLimiter::new(config).unwrap();

        // Admin should bypass
        let result = limiter.check("admin", None).await;
        assert!(result.allowed);
        assert_eq!(result.remaining, u64::MAX);

        // Regular client should be limited after first request
        let result = limiter.check("client1", None).await;
        assert!(result.allowed);
        let result = limiter.check("client1", None).await;
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_different_clients() {
        let config = RateLimiterConfig::default();
        let limiter = RateLimiter::new(config).unwrap();

        // Different clients have independent limits
        limiter.check("client1", None).await;
        limiter.check("client2", None).await;

        let status1 = limiter.status("client1", None).await;
        let status2 = limiter.status("client2", None).await;

        assert_eq!(status1.remaining, status2.remaining);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let config = RateLimiterConfig {
            enabled: true,
            default_tier: RateLimitTier {
                name: "test".to_string(),
                max_requests: 2,
                window_seconds: 60,
                burst_capacity: None,
            },
            tiers: Vec::new(),
            ..Default::default()
        };

        let limiter = RateLimiter::new(config).unwrap();

        limiter.check("client1", None).await;
        limiter.check("client1", None).await;
        let result = limiter.check("client1", None).await;
        assert!(!result.allowed);

        limiter.reset("client1").await;
        let result = limiter.check("client1", None).await;
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_invalid_config() {
        let config = RateLimiterConfig {
            enabled: true,
            default_tier: RateLimitTier {
                name: "test".to_string(),
                max_requests: 0,
                window_seconds: 60,
                burst_capacity: None,
            },
            tiers: Vec::new(),
            ..Default::default()
        };

        let result = RateLimiter::new(config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_tier_override() {
        let config = RateLimiterConfig {
            enabled: true,
            default_tier: RateLimitTier {
                name: "default".to_string(),
                max_requests: 5,
                window_seconds: 60,
                burst_capacity: None,
            },
            tiers: vec![RateLimitTier {
                name: "premium".to_string(),
                max_requests: 100,
                window_seconds: 60,
                burst_capacity: None,
            }],
            ..Default::default()
        };

        let limiter = RateLimiter::new(config).unwrap();

        // Use premium tier - should allow many more requests
        for i in 1..=10 {
            let result = limiter.check("client1", Some("premium")).await;
            assert!(result.allowed, "Request {} should be allowed with premium tier", i);
        }
    }

    #[test]
    fn test_rate_limit_result_serialization() {
        let result = RateLimitResult {
            allowed: true,
            remaining: 10,
            limit: 100,
            reset_seconds: 30,
            retry_after: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"allowed\":true"));
        assert!(json.contains("\"remaining\":10"));
    }
}

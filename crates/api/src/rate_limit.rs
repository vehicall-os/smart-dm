//! Rate Limiting Middleware using GCRA Algorithm
//!
//! Provides rate limiting based on IP addresses using tower_governor.
//! Uses the Generic Cell Rate Algorithm (GCRA) for efficient,
//! accurate rate enforcement without background processes.

use governor::middleware::StateInformationMiddleware;
use std::sync::Arc;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::PeerIpKeyExtractor;

/// Type alias for the governor config with default settings
/// StateInformationMiddleware is used when use_headers() is called to add X-RateLimit-* headers
pub type DefaultGovernorConfig =
    tower_governor::governor::GovernorConfig<PeerIpKeyExtractor, StateInformationMiddleware>;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests allowed per second (replenishment rate)
    pub per_second: u64,
    /// Burst size (max requests that can be made immediately)
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            per_second: 2,  // Replenish every 2 seconds
            burst_size: 5,  // Allow burst of 5
        }
    }
}

impl RateLimitConfig {
    /// Create a strict config for sensitive endpoints
    pub fn strict() -> Self {
        Self {
            per_second: 4,  // One request every 4 seconds
            burst_size: 2,
        }
    }

    /// Create a lenient config for public endpoints  
    pub fn lenient() -> Self {
        Self {
            per_second: 1,  // Replenish 1 per second
            burst_size: 10,
        }
    }
}

/// Create a rate limiting governor config 
/// 
/// Returns an Arc wrapped config that can be used with GovernorLayer.
/// Uses PeerIpKeyExtractor by default. Requires service to use
/// `into_make_service_with_connect_info::<SocketAddr>()` for IP extraction.
/// 
/// Adds X-RateLimit-* headers to responses for quota visibility.
pub fn create_governor_config(config: &RateLimitConfig) -> Arc<DefaultGovernorConfig> {
    Arc::new(
        GovernorConfigBuilder::default()
            .per_second(config.per_second)
            .burst_size(config.burst_size)
            .use_headers()  // Adds X-RateLimit-After, X-RateLimit-Limit, X-RateLimit-Remaining
            .finish()
            .unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.per_second, 2);
        assert_eq!(config.burst_size, 5);
    }

    #[test]
    fn test_strict_config() {
        let config = RateLimitConfig::strict();
        assert_eq!(config.per_second, 4);
        assert_eq!(config.burst_size, 2);
    }
    
    #[test]
    fn test_create_governor_config() {
        let config = RateLimitConfig::default();
        let governor = create_governor_config(&config);
        // Just verify it doesn't panic
        assert!(Arc::strong_count(&governor) > 0);
    }
}

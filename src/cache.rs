//! Package cache model.

/// Placeholder for package cache configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheConfig {
    /// Whether cache operations may write to disk.
    pub writable: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self { writable: true }
    }
}

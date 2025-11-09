//! Entry structure for key-value pairs

use super::value::Value;
use bytes::Bytes;
use std::time::{Duration, Instant};

/// Represents a single entry in the store
#[derive(Debug, Clone)]
pub struct Entry {
    /// The key
    pub key: Bytes,

    /// The value
    pub value: Value,

    /// Optional expiration time (absolute)
    pub expire_at: Option<Instant>,

    /// Version number for optimistic concurrency control (future use)
    pub version: u64,
}

impl Entry {
    /// Create a new entry without expiration
    pub fn new(key: impl Into<Bytes>, value: Value) -> Self {
        Entry {
            key: key.into(),
            value,
            expire_at: None,
            version: 0,
        }
    }

    /// Create a new entry with expiration
    pub fn with_expiration(
        key: impl Into<Bytes>,
        value: Value,
        ttl: Duration,
    ) -> Self {
        Entry {
            key: key.into(),
            value,
            expire_at: Some(Instant::now() + ttl),
            version: 0,
        }
    }

    /// Check if the entry has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expire_at) = self.expire_at {
            Instant::now() >= expire_at
        } else {
            false
        }
    }

    /// Set expiration time (TTL in seconds)
    pub fn set_expiration(&mut self, ttl_seconds: i64) {
        if ttl_seconds > 0 {
            self.expire_at = Some(Instant::now() + Duration::from_secs(ttl_seconds as u64));
        } else {
            self.expire_at = None;
        }
    }

    /// Remove expiration
    pub fn remove_expiration(&mut self) {
        self.expire_at = None;
    }

    /// Get remaining TTL in seconds
    pub fn ttl_seconds(&self) -> i64 {
        match self.expire_at {
            Some(expire_at) => {
                let now = Instant::now();
                if expire_at > now {
                    expire_at.duration_since(now).as_secs() as i64
                } else {
                    -2 // Expired
                }
            }
            None => -1, // No expiration
        }
    }

    /// Increment version (for future multi-node synchronization)
    pub fn increment_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    /// Calculate approximate memory usage of this entry in bytes
    pub fn memory_usage(&self) -> usize {
        let key_size = self.key.len();
        let value_size = self.value.memory_usage();
        let metadata_size = std::mem::size_of::<Option<Instant>>() + std::mem::size_of::<u64>();
        key_size + value_size + metadata_size
    }
}

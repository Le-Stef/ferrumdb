//! In-memory storage implementation

use super::entry::Entry;
use super::value::Value;
use bytes::Bytes;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use siphasher::sip::SipHasher13;

/// Type alias for our hash map with SipHasher
type StoreMap = HashMap<Bytes, Entry, BuildHasherDefault<SipHasher13>>;

/// In-memory key-value store
///
/// This is the core storage engine. For Phase 1 (MVP), this is a simple
/// single-threaded HashMap. Future phases will add sharding.
pub struct MemoryStore {
    /// The main storage map
    store: StoreMap,

    /// Total number of keys (including expired)
    total_keys: usize,

    /// Number of expired keys that haven't been cleaned up yet
    expired_keys: usize,
}

impl MemoryStore {
    /// Create a new memory store with default capacity
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a new memory store with specified initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        MemoryStore {
            store: HashMap::with_capacity_and_hasher(
                capacity,
                BuildHasherDefault::<SipHasher13>::default(),
            ),
            total_keys: 0,
            expired_keys: 0,
        }
    }

    /// Set a key-value pair
    pub fn set(&mut self, key: impl Into<Bytes>, value: Value) -> bool {
        let key = key.into();
        let entry = Entry::new(key.clone(), value);
        let is_new = !self.store.contains_key(&key);

        self.store.insert(key, entry);

        if is_new {
            self.total_keys += 1;
        }

        is_new
    }

    /// Get a value by key, returns None if not found or expired
    pub fn get(&mut self, key: &Bytes) -> Option<&Value> {
        // First check if key exists and if it's expired
        let is_expired = self.store.get(key)
            .map(|entry| entry.is_expired())
            .unwrap_or(false);

        if is_expired {
            self.expired_keys += 1;
            self.store.remove(key);
            return None;
        }

        // Now get the value reference
        self.store.get(key).map(|entry| &entry.value)
    }

    /// Get a mutable reference to a value by key
    pub fn get_mut(&mut self, key: &Bytes) -> Option<&mut Value> {
        // Check if key exists and not expired
        if let Some(entry) = self.store.get(key) {
            if entry.is_expired() {
                self.expired_keys += 1;
                self.store.remove(key);
                return None;
            }
        }

        // Now get mutable reference
        self.store.get_mut(key).map(|entry| &mut entry.value)
    }

    /// Delete a key, returns true if the key existed
    pub fn delete(&mut self, key: &Bytes) -> bool {
        if let Some(entry) = self.store.remove(key) {
            if !entry.is_expired() {
                self.total_keys -= 1;
                true
            } else {
                self.expired_keys -= 1;
                false
            }
        } else {
            false
        }
    }

    /// Check if a key exists (and is not expired)
    pub fn exists(&mut self, key: &Bytes) -> bool {
        if let Some(entry) = self.store.get(key) {
            if entry.is_expired() {
                self.expired_keys += 1;
                self.store.remove(key);
                return false;
            }
            return true;
        }
        false
    }

    /// Set expiration on a key (TTL in seconds)
    pub fn expire(&mut self, key: &Bytes, ttl_seconds: i64) -> bool {
        if let Some(entry) = self.store.get_mut(key) {
            if entry.is_expired() {
                self.expired_keys += 1;
                self.store.remove(key);
                return false;
            }
            entry.set_expiration(ttl_seconds);
            return true;
        }
        false
    }

    /// Get TTL for a key in seconds
    /// Returns:
    /// - Some(n) where n >= 0: remaining TTL in seconds
    /// - Some(-1): key exists but has no expiration
    /// - Some(-2): key does not exist or is expired
    pub fn ttl(&mut self, key: &Bytes) -> i64 {
        if let Some(entry) = self.store.get(key) {
            if entry.is_expired() {
                self.expired_keys += 1;
                self.store.remove(key);
                return -2;
            }
            return entry.ttl_seconds();
        }
        -2 // Key not found
    }

    /// Get the entry for a key (including expiration metadata)
    pub fn get_entry(&self, key: &Bytes) -> Option<&Entry> {
        self.store.get(key)
    }

    /// Get a mutable entry reference
    pub fn get_entry_mut(&mut self, key: &Bytes) -> Option<&mut Entry> {
        self.store.get_mut(key)
    }

    /// Remove all keys
    pub fn clear(&mut self) {
        self.store.clear();
        self.total_keys = 0;
        self.expired_keys = 0;
    }

    /// Get the number of active keys (excluding expired)
    pub fn len(&self) -> usize {
        self.total_keys.saturating_sub(self.expired_keys)
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all keys (expensive operation, for debugging/admin)
    pub fn keys(&self) -> Vec<Bytes> {
        self.store
            .values()
            .filter(|entry| !entry.is_expired())
            .map(|entry| entry.key.clone())
            .collect()
    }

    /// Cleanup expired keys (proactive expiration)
    /// Returns the number of keys removed
    pub fn cleanup_expired(&mut self) -> usize {
        let mut removed = 0;
        let keys_to_remove: Vec<Bytes> = self.store
            .values()
            .filter(|entry| entry.is_expired())
            .map(|entry| entry.key.clone())
            .collect();

        for key in keys_to_remove {
            self.store.remove(&key);
            removed += 1;
        }

        self.expired_keys = self.expired_keys.saturating_sub(removed);
        self.total_keys = self.total_keys.saturating_sub(removed);
        removed
    }

    /// Calculate approximate memory usage of stored data in bytes
    pub fn memory_usage(&self) -> usize {
        self.store
            .values()
            .filter(|entry| !entry.is_expired())
            .map(|entry| entry.memory_usage())
            .sum()
    }

    /// Get statistics about the store
    pub fn stats(&self) -> StoreStats {
        // Count actual non-expired keys (not relying on lazy deletion counter)
        let active_count = self.store
            .values()
            .filter(|entry| !entry.is_expired())
            .count();

        // Count expired keys that haven't been cleaned up yet
        let expired_count = self.store.len() - active_count;

        StoreStats {
            total_keys: self.store.len(),
            expired_keys: expired_count,
            active_keys: active_count,
            used_memory_bytes: self.memory_usage(),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the memory store
#[derive(Debug, Clone)]
pub struct StoreStats {
    pub total_keys: usize,
    pub expired_keys: usize,
    pub active_keys: usize,
    pub used_memory_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_set_get() {
        let mut store = MemoryStore::new();
        store.set("key1", Value::string("value1"));

        let value = store.get(&Bytes::from("key1")).unwrap();
        assert_eq!(value.as_string().unwrap(), &Bytes::from("value1"));
    }

    #[test]
    fn test_delete() {
        let mut store = MemoryStore::new();
        store.set("key1", Value::string("value1"));

        assert!(store.delete(&Bytes::from("key1")));
        assert!(!store.exists(&Bytes::from("key1")));
    }

    #[test]
    fn test_exists() {
        let mut store = MemoryStore::new();
        store.set("key1", Value::string("value1"));

        assert!(store.exists(&Bytes::from("key1")));
        assert!(!store.exists(&Bytes::from("key2")));
    }

    #[test]
    fn test_expiration() {
        let mut store = MemoryStore::new();
        store.set("key1", Value::string("value1"));
        store.expire(&Bytes::from("key1"), 1);

        assert!(store.exists(&Bytes::from("key1")));

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_secs(2));

        assert!(!store.exists(&Bytes::from("key1")));
    }
}

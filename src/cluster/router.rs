//! Key routing logic for sharding
//!
//! Routes keys to shards using consistent hashing based on SipHash.

use bytes::Bytes;
use std::hash::{Hash, Hasher};
use siphasher::sip::SipHasher13;

/// Routes keys to shards using consistent hashing
pub struct ShardRouter {
    num_shards: usize,
}

impl ShardRouter {
    /// Create a new shard router
    pub fn new(num_shards: usize) -> Self {
        assert!(num_shards > 0, "Number of shards must be > 0");
        ShardRouter { num_shards }
    }

    /// Route a key to a shard ID
    ///
    /// Uses SipHash13 for fast, secure hashing with good distribution.
    /// This ensures keys are evenly distributed across shards.
    pub fn route_key(&self, key: &Bytes) -> usize {
        let hash = self.hash_key(key);
        (hash as usize) % self.num_shards
    }

    /// Hash a key using SipHash13
    fn hash_key(&self, key: &Bytes) -> u64 {
        let mut hasher = SipHasher13::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Get the number of shards
    pub fn num_shards(&self) -> usize {
        self.num_shards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_deterministic() {
        let router = ShardRouter::new(4);
        let key = Bytes::from("test_key");

        // Same key should always route to same shard
        let shard1 = router.route_key(&key);
        let shard2 = router.route_key(&key);
        assert_eq!(shard1, shard2);
    }

    #[test]
    fn test_routing_distribution() {
        let router = ShardRouter::new(4);
        let mut shard_counts = vec![0; 4];

        // Test with 1000 keys
        for i in 0..1000 {
            let key = Bytes::from(format!("key_{}", i));
            let shard = router.route_key(&key);
            shard_counts[shard] += 1;
        }

        // Each shard should get roughly 250 keys (±50 for variance)
        for count in shard_counts {
            assert!(count > 200 && count < 300, "Uneven distribution: {}", count);
        }
    }

    #[test]
    fn test_single_shard() {
        let router = ShardRouter::new(1);
        let key = Bytes::from("any_key");
        assert_eq!(router.route_key(&key), 0);
    }
}

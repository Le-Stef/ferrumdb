//! AOF replay
//!
//! Handles replaying AOF entries to reconstruct the database state.

use super::{AofEntry, AofOperation};
use crate::store::{MemoryStore, Value};
//use bytes::Bytes;
//use std::collections::HashMap;
use tracing::{info, warn};

/// Replay AOF entries into a memory store
///
/// This function takes a vector of AOF entries and replays them
/// to reconstruct the database state.
pub fn replay_entries(store: &mut MemoryStore, entries: Vec<AofEntry>) -> Result<usize, String> {
    let mut replayed = 0;

    for entry in entries {
        match replay_entry(store, &entry) {
            Ok(()) => replayed += 1,
            Err(e) => {
                warn!("Failed to replay AOF entry: {}. Skipping.", e);
                // Continue with next entry
            }
        }
    }

    info!("Successfully replayed {} AOF entries", replayed);
    Ok(replayed)
}

/// Replay a single AOF entry
fn replay_entry(store: &mut MemoryStore, entry: &AofEntry) -> Result<(), String> {
    match entry.op {
        AofOperation::Set => {
            if entry.payload.is_empty() {
                return Err("SET operation requires value payload".to_string());
            }
            let value = &entry.payload[0];
            store.set(entry.key.clone(), Value::String(value.clone()));
            Ok(())
        }

        AofOperation::Del => {
            store.delete(&entry.key);
            Ok(())
        }

        AofOperation::Expire => {
            if entry.payload.is_empty() {
                return Err("EXPIRE operation requires TTL payload".to_string());
            }
            let ttl_str = std::str::from_utf8(&entry.payload[0])
                .map_err(|_| "Invalid TTL encoding")?;
            let ttl: i64 = ttl_str.parse()
                .map_err(|_| "Invalid TTL value")?;
            store.expire(&entry.key, ttl);
            Ok(())
        }

        AofOperation::HSet => {
            if entry.payload.len() < 2 {
                return Err("HSET operation requires field and value".to_string());
            }
            let field = &entry.payload[0];
            let value = &entry.payload[1];

            // Get or create hash
            let hash_value = match store.get_mut(&entry.key) {
                Some(v) => {
                    match v.as_hash_mut() {
                        Some(h) => h,
                        None => return Err("Key exists but is not a hash".to_string()),
                    }
                }
                None => {
                    store.set(entry.key.clone(), Value::empty_hash());
                    store.get_mut(&entry.key).unwrap().as_hash_mut().unwrap()
                }
            };

            hash_value.insert(field.clone(), value.clone());
            Ok(())
        }

        AofOperation::HDel => {
            if entry.payload.is_empty() {
                return Err("HDEL operation requires field".to_string());
            }
            let field = &entry.payload[0];

            if let Some(value) = store.get_mut(&entry.key) {
                if let Some(hash) = value.as_hash_mut() {
                    hash.remove(field);
                }
            }
            Ok(())
        }

        AofOperation::LPush => {
            // TODO: Implement list operations
            warn!("LPUSH replay not yet implemented");
            Ok(())
        }

        AofOperation::RPush => {
            // TODO: Implement list operations
            warn!("RPUSH replay not yet implemented");
            Ok(())
        }

        AofOperation::SAdd => {
            // TODO: Implement set operations
            warn!("SADD replay not yet implemented");
            Ok(())
        }

        AofOperation::Incr => {
            // INCR is replayed as SET
            if entry.payload.is_empty() {
                return Err("INCR operation requires value payload".to_string());
            }
            let value_str = std::str::from_utf8(&entry.payload[0])
                .map_err(|_| "Invalid value encoding")?;
            let value: i64 = value_str.parse()
                .map_err(|_| "Invalid integer value")?;
            store.set(entry.key.clone(), Value::Integer(value));
            Ok(())
        }

        AofOperation::IncrBy => {
            // INCRBY is replayed as SET
            if entry.payload.is_empty() {
                return Err("INCRBY operation requires value payload".to_string());
            }
            let value_str = std::str::from_utf8(&entry.payload[0])
                .map_err(|_| "Invalid value encoding")?;
            let value: i64 = value_str.parse()
                .map_err(|_| "Invalid integer value")?;
            store.set(entry.key.clone(), Value::Integer(value));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_set() {
        let mut store = MemoryStore::new();

        let entry = AofEntry::new(
            AofOperation::Set,
            Bytes::from("key1"),
            vec![Bytes::from("value1")],
        );

        replay_entry(&mut store, &entry).unwrap();

        assert!(store.exists(&Bytes::from("key1")));
        let value = store.get(&Bytes::from("key1")).unwrap();
        assert_eq!(value.as_string().unwrap(), &Bytes::from("value1"));
    }

    #[test]
    fn test_replay_hset() {
        let mut store = MemoryStore::new();

        let entry = AofEntry::new(
            AofOperation::HSet,
            Bytes::from("myhash"),
            vec![Bytes::from("field1"), Bytes::from("value1")],
        );

        replay_entry(&mut store, &entry).unwrap();

        assert!(store.exists(&Bytes::from("myhash")));
        let value = store.get(&Bytes::from("myhash")).unwrap();
        let hash = value.as_hash().unwrap();
        assert_eq!(hash.get(&Bytes::from("field1")).unwrap(), &Bytes::from("value1"));
    }

    #[test]
    fn test_replay_multiple_entries() {
        let mut store = MemoryStore::new();

        let entries = vec![
            AofEntry::new(
                AofOperation::Set,
                Bytes::from("key1"),
                vec![Bytes::from("value1")],
            ),
            AofEntry::new(
                AofOperation::Set,
                Bytes::from("key2"),
                vec![Bytes::from("value2")],
            ),
            AofEntry::new(
                AofOperation::Del,
                Bytes::from("key1"),
                vec![],
            ),
        ];

        let replayed = replay_entries(&mut store, entries).unwrap();
        assert_eq!(replayed, 3);

        assert!(!store.exists(&Bytes::from("key1")));
        assert!(store.exists(&Bytes::from("key2")));
    }
}

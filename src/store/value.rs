//! Value types for the key-value store

use bytes::Bytes;
use std::collections::{HashMap, HashSet, VecDeque};

/// Represents the different types of values that can be stored
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// String value (binary-safe)
    String(Bytes),

    /// Integer value (used for counters)
    Integer(i64),

    /// List of values (ordered)
    List(VecDeque<Bytes>),

    /// Set of unique values (unordered)
    Set(HashSet<Bytes>),

    /// Hash map (field -> value)
    Hash(HashMap<Bytes, Bytes>),

    // TODO Phase 2+: ZSet (sorted set), Bitmap, etc.
}

impl Value {
    /// Create a string value
    pub fn string(bytes: impl Into<Bytes>) -> Self {
        Value::String(bytes.into())
    }

    /// Create an integer value
    pub fn integer(i: i64) -> Self {
        Value::Integer(i)
    }

    /// Create an empty list
    pub fn empty_list() -> Self {
        Value::List(VecDeque::new())
    }

    /// Create an empty set
    pub fn empty_set() -> Self {
        Value::Set(HashSet::new())
    }

    /// Create an empty hash
    pub fn empty_hash() -> Self {
        Value::Hash(HashMap::new())
    }

    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Integer(_) => "integer",
            Value::List(_) => "list",
            Value::Set(_) => "set",
            Value::Hash(_) => "hash",
        }
    }

    /// Check if value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if value is an integer
    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }

    /// Try to get as string bytes
    pub fn as_string(&self) -> Option<&Bytes> {
        match self {
            Value::String(b) => Some(b),
            _ => None,
        }
    }

    /// Try to get as integer
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as mutable list
    pub fn as_list_mut(&mut self) -> Option<&mut VecDeque<Bytes>> {
        match self {
            Value::List(list) => Some(list),
            _ => None,
        }
    }

    /// Try to get as mutable set
    pub fn as_set_mut(&mut self) -> Option<&mut HashSet<Bytes>> {
        match self {
            Value::Set(set) => Some(set),
            _ => None,
        }
    }

    /// Try to get as mutable hash
    pub fn as_hash_mut(&mut self) -> Option<&mut HashMap<Bytes, Bytes>> {
        match self {
            Value::Hash(hash) => Some(hash),
            _ => None,
        }
    }

    /// Try to get as hash reference
    pub fn as_hash(&self) -> Option<&HashMap<Bytes, Bytes>> {
        match self {
            Value::Hash(hash) => Some(hash),
            _ => None,
        }
    }

    /// Try to get as list reference
    pub fn as_list(&self) -> Option<&VecDeque<Bytes>> {
        match self {
            Value::List(list) => Some(list),
            _ => None,
        }
    }

    /// Try to get as set reference
    pub fn as_set(&self) -> Option<&HashSet<Bytes>> {
        match self {
            Value::Set(set) => Some(set),
            _ => None,
        }
    }

    /// Calculate approximate memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        match self {
            Value::String(bytes) => bytes.len(),
            Value::Integer(_) => std::mem::size_of::<i64>(),
            Value::List(list) => {
                let items_size: usize = list.iter().map(|b| b.len()).sum();
                let overhead = std::mem::size_of::<VecDeque<Bytes>>();
                items_size + overhead
            }
            Value::Set(set) => {
                let items_size: usize = set.iter().map(|b| b.len()).sum();
                let overhead = std::mem::size_of::<HashSet<Bytes>>();
                items_size + overhead
            }
            Value::Hash(hash) => {
                let items_size: usize = hash.iter()
                    .map(|(k, v)| k.len() + v.len())
                    .sum();
                let overhead = std::mem::size_of::<HashMap<Bytes, Bytes>>();
                items_size + overhead
            }
        }
    }
}

// Implement Eq and Hash for Bytes to allow it in HashSet
// (Bytes already implements these, but we make it explicit for clarity)
impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Value::String(b) => {
                0u8.hash(state);
                b.hash(state);
            }
            Value::Integer(i) => {
                1u8.hash(state);
                i.hash(state);
            }
            Value::List(l) => {
                2u8.hash(state);
                l.len().hash(state);
            }
            Value::Set(s) => {
                3u8.hash(state);
                s.len().hash(state);
            }
            Value::Hash(h) => {
                4u8.hash(state);
                h.len().hash(state);
            }
        }
    }
}
